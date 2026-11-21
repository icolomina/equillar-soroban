---
sidebar_position: 7
title: Reembolsos (refunds)
---

# Reembolsos (refunds)

Los reembolsos solo se permiten durante el periodo de fundraising, y solo cuando el fundraising del proyecto se cancela. Esto puede ocurrir, por ejemplo, cuando la empresa determina que el capital recaudado está lejos del objetivo y que es improbable alcanzarlo en el tiempo de financiación restante.

## La función refund_investor

El admin puede reembolsar una posición mediante `refund_investor(token_id)`:

```bash
stellar contract invoke \
  --id <YOUR_CONTRACT_ID> \
  -s admin \
  --network testnet \
  -- \
  refund_investor \
  --token_id 1234
```

> **Nota de acceso:** a pesar de cerrar una posición creada por un operator, se trata de una función **solo-admin**.

Los reembolsos solo son válidos si se cumplen **todas** las siguientes condiciones:

| Comprobación | Error si se no se cumple |
|---|---|
| La posición existe | `AddressHasNotInvested` (14) |
| La marca de tiempo del ledger actual es anterior a `ts_fundraising_ends` | `FundrasingTimeExceeded` (40) |
| La posición no está completada | `PositionCompleted` (38) |
| El importe de reembolso es distinto de cero | `EmptyRefundAmount` (46) |
| El contrato tiene suficiente liquidez para pagarlo | `ContractInsufficientBalance` (3) |

## El importe del reembolso

El reembolso devuelve exactamente el capital que el inversor transfirió originalmente — el **importe bruto**, no la asignación. Dado que el monto depositado se calcula como:

```text
deposited = amount recibido (libre de truncamientos) - comision
```

El total devuelto siempre será exactamente aquello que el inversor aportó: `total a retornar = deposited + comisión`
La posición se marca entonces como `completed` y se aplican los ajustes contables agregados.

## Contabilidad: reversión completa de la comisión

`recalculate_from_refunded_to_investor` aplica cuatro movimientos y verifica la ecuación:

- `assets.managed_volume -= total` (El volumen negociado se reduce dado que se ha devuelto el depósito al inversor)
- `liabilities.payment_obligations -= total` (Las obligaciones de pago se reducen dado que la posición desaparece)
- `assets.contract_earnings -= commission` (La comisión también se reduce dado que también se devuelve al inversor)
- `equity.contract_commissions -= commission` (El patrimonio neto se reduce dado que la comisión se devuelve al inversor)

## Eventos y balance tras un reembolso

**1. `BalanceUpdated`**: el snapshot de balance actualizado:

```rust
BalanceUpdated {
    assets: BalanceAsset { payment_rigths, contract_earnings, managed_volume, managed_collateral },
    liabilities: BalanceLiabilities { payment_obligations, return_collateral_obligations },
    equity: BalanceEquity { contract_commissions },
}
```

**2. `InvestmentDepositRefunded`**: Información del reembolso

```rust
InvestmentDepositRefunded {
    addr,     // la dirección del contrato
    to,       // la dirección a la que se reembolsa el depósito
    amount,   // el importe total reembolsado (deposited + commission)
}
```

### Leer este evento con stellar-cli

```bash
stellar events \
  --id <CONTRACT_ID> \
  --network testnet \
  --start-ledger <LEDGER_NUMBER> \
  --type contract \
  --topic "InvestmentDepositRefunded,*"
```

## Reembolsos frente a cierre de emergencia

> **No** es lo mismo que el [cierre de emergencia](./emergency-close.md): `refund_investor` devuelve el importe depositado al inversor cuando el contrato aun puede recibir depósitos (en ese momento la empresa aún no ha empezado a operar y por tanto no ha empezado a gastar los fondos recaudados). El cierre de emergencia es el camino posterior al fundraising y se produce cuando la empresa no puede cumplir con sus obligaciones de pago.
