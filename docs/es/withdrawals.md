---
sidebar_position: 9
title: Retiradas
---

# Retiradas

Las retiradas se permiten tras terminar el periodo de fundraising. Equillar separa las **retiradas de proyecto** de las **retiradas de comisiones**, además de añadir una operación final de barrido total.

> **Nota TTL — la actividad puede caer durante el claim block:** durante la ventana de claim block (tras el fundraising, antes de `ts_payments_start`) no se aceptan nuevas inversiones y las rondas de pago aún no han comenzado. La actividad más notable son las retiradas (tanto de la empresa como de comisiones). Esta actividad reducida puede provocar que las entradas del ledger que almacenan posiciones, balances y otros datos del contrato se archiven por expiración del TTL (time-to-live). Desde el Protocolo 23, las entradas archivadas en almacenamiento Persistent e Instance pueden restaurarse automáticamente como parte de la ejecución de una transacción, siempre que la entrada esté incluida en el restore footprint de la transacción lo cual, en la práctica, se rellena automáticamente cuando la invocación se simula vía Stellar RPC (el flujo estándar usado por stellar-cli y la mayoría de SDKs/integradores).
>
> Equillar no implementa un mecanismo proactivo de extensión de TTL (p. ej. una llamada programada tipo "keeper" para renovar TTLs durante ventanas largas de inactividad). En su lugar, el contrato confía en la restauración automática del Protocolo 23 durante la invocación normal. La implicación práctica para los integradores es que la primera llamada tras un periodo prolongado de inactividad puede incurrir en fees mayores, debido al coste on-chain de restaurar las entradas archivadas, pero la llamada en sí no falla ni requiere un paso manual de restore separado, siempre que pase por un cliente que simule la transacción previamente. Si un integrador envía una transacción con un footprint construido manualmente (eludiendo la simulación RPC) contra una entrada archivada, esa transacción fallará en lugar de autorestaurarse.

## Retiradas de proyecto de la empresa

El admin puede retirar fondos del proyecto del contrato mediante la función `withdrawn(amount, to)`:

```bash
stellar contract invoke \
  --id <YOUR_CONTRACT_ID> \
  -s admin \
  --network testnet \
  -- \
  withdrawn \
  --amount "3000000000" \
  --to "G....."
```

Las retiradas de empresa solo se permiten si se cumplen todas las condiciones siguientes:

| Comprobación | Error si no se cumple |
|---|---|
| El contrato no está pausado / no hay emergencia activa | guard de pausa / `OperationNotAllowedInEmergency` (50) |
| El periodo de financiación ha finalizado | `FundrasingTimeOngoingYet` (39) |
| La dirección a la cual enviar los fondos posee el rol `company` | error de rol |
| El contrato tiene fondos suficientes | `ContractInsufficientBalance` (3) |

El balance de proyecto es lo que la empresa puede retirar: la suma de todas las inversiones de los usuarios **menos las comisiones**. Siguiendo el ejemplo de [Fundraising y posiciones](./fundraising-and-positions.md#ejemplo-a-escala-del-token) (una inversión de `500` tokens), el importe disponible para el proyecto es `497.7272728` tokens, quedando el resto asignado al bucket de comisiones.

## Retiradas de comisiones

El admin puede retirar las comisiones acumuladas mediante `withdrawn_commissions(to)`:

```bash
stellar contract invoke \
  --id <YOUR_CONTRACT_ID> \
  -s admin \
  --network testnet \
  -- \
  withdrawn_commissions \
  --to "G....."
```

Las retiradas de comisiones solo se permiten si se cumplen todas las condiciones siguientes:

| Comprobación | Error si no se cumple |
|---|---|
| El contrato no está pausado / no hay emergencia activa | guard de pausa / (50) |
| El periodo de financiación ha finalizado | `FundrasingTimeOngoingYet` (39) |
| La dirección a la cual enviar los fondos posee el rol `manager` | error de rol |
| Hay comisión pendiente (`equity.contract_commissions > 0`) | `AmountLessThanMinimum` (5) |
| La liquidez del contrato cubre la retirada | `ContractInsufficientBalance` (3) |

Observa que `withdrawn_commissions` **no** requiere un importe: retira la **totalidad** de la comisión pendiente (`equity.contract_commissions`). El integrador nunca necesita trackear cuánto tiene disponible. Las llamadas a esta función deben espaciarse lo suficiente como para que cada vez haya comisiones disponibles, evitando llamadas fallidas innecesarias.

En términos contables, `recalculate_from_comission_withdrawal` aplica la retirada simétricamente en ambos lados del balance: `assets.contract_earnings -= amount` y `equity.contract_commissions -= amount` de manera que las comisiones retiradas salen completamente de la contabilidad (no se "conservan acumuladas", a diferencia del modelo anterior).

## Barrido final — withdrawn_all

El admin puede barrer **todos los fondos de token restantes** del contrato hacia una dirección con el rol `manager`, llamando a `withdrawn_all(to)`:

```bash
stellar contract invoke \
  --id <YOUR_CONTRACT_ID> \
  -s admin \
  --network testnet \
  -- \
  withdrawn_all \
  --to "G....."
```

Es la operación de cierre del ciclo de vida de una instancia. Sus condiciones previas, comprobadas en este orden:

| Comprobación | Error si no se cumple |
|---|---|
| El contrato no mantiene colateral pendiente de retirar | `CollateralStillRemainingOnContract` (60) |
| El cierre de emergencia está **activo** | `EmergencyNotActive` (49) |

Si el contrato aún mantiene tokens, la totalidad del balance del token se transfiere a `to`, y el `Balance` se **resetea** (todos los campos a cero). Las obligaciones de pago **no** necesitan estar a cero: el integrador decide off-chain qué hacer con los fondos recibidos pero el **colateral debe haberse retirado previamente** mediante [`withdrawn_all_collateral`](./collateral.md#barrido-final-del-colateral--withdrawn_all_collateral): con colateral aún en el contrato, `withdrawn_all` se rechaza con `CollateralStillRemainingOnContract` (60).

> `withdrawn_all` exige que la compuerta de emergencia esté activa. En la sección [Cierre de emergencia](./emergency-close.md) se analiza cómo la activación y la operación componen juntas el cierre de una instancia. Para el colateral restante existe la operación equivalente [`withdrawn_all_collateral`](./collateral.md#barrido-final-del-colateral--withdrawn_all_collateral), que también envía los fondos a una dirección `manager` en lugar de devolverlos al proveedor.

## Eventos y balance tras una retirada de empresa

Cuando la empresa retira fondos de proyecto mediante `withdrawn`, el balance se actualiza vía `recalculate_from_company_withdrawal`:

- `assets.payment_rigths += amount` (Aumentan las obligaciones de pago de la empresa con el contrato)
- `assets.managed_volume -= amount` (El volumen gestionado se reduce)

Esta actualización se publica a través del evento estándar `BalanceUpdated`. Además, el contrato emite un evento dedicado con los detalles de la retirada:

**`WithdrawalDone`**:

```rust
WithdrawalDone {
    addr,              // la dirección del contrato
    total_withdrawn,   // el importe total retirado por la empresa
}
```

### Leer este evento con stellar-cli

```bash
stellar events \
  --id <CONTRACT_ID> \
  --network testnet \
  --start-ledger <LEDGER_NUMBER> \
  --type contract \
  --topic "WithdrawalDone,*"
```

## Eventos y balance tras una retirada de comisiones

Cuando el integrador retira las comisiones acumuladas mediante `withdrawn_commissions`, la actualización del balance decrece tanto el activo de comisiones como el capital pendiente.

**`CommissionWithdrawn`**:

```rust
CommissionWithdrawn {
    addr,     // la dirección del contrato
    amount,   // el importe total de comisiones retiradas
}
```

### Leer este evento con stellar-cli

```bash
stellar events \
  --id <CONTRACT_ID> \
  --network testnet \
  --start-ledger <LEDGER_NUMBER> \
  --type contract \
  --topic "CommissionWithdrawn,*"
```

## Eventos y balance tras retirar todos los fondos

Cuando el admin barre todos los fondos restantes mediante `withdrawn_all`, el balance se resetea por completo vía `balance.reset()` y todos los parámetros de la ecuación vuelven a `0`:

```text
assets:       payment_rigths, contract_earnings, managed_volume, managed_collateral = 0
liabilities:  payment_obligations, return_collateral_obligations                = 0
equity:       contract_commissions                                              = 0
```

Esta operación también emite el evento estándar `BalanceUpdated` (con el balance ya reseteado); además emite el evento dedicado que registra el barrido:

**`AllWithdrawn`**:

```rust
AllWithdrawn {
    addr,     // la dirección del contrato
    amount,   // el importe total retirado
}
```

### Leer este evento con stellar-cli

```bash
stellar events \
  --id <CONTRACT_ID> \
  --network testnet \
  --start-ledger <LEDGER_NUMBER> \
  --type contract \
  --topic "AllWithdrawn,*"
```
