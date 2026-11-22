---
sidebar_position: 4
title: El modelo de Balance
---

# El modelo de Balance (contabilidad de partida doble)

La estructura `Balance` es el corazón financiero de Equillar. No es un único número: es un **snapshot de contabilidad de partida doble** que trackea, en todo momento, qué gestiona el contrato (activos), qué debe (pasivos) y qué ha ganado el integrador (capital).

Cada operación con cambio de estado actualiza el Balance mediante una función dedicada `recalculate_from_*`, y cada una **verifica la ecuación contable antes de persistir**. Si la ecuación se rompe, la operación revierte con un error de negocio en lugar de dejar un estado inconsistente on-chain.

## La ecuación contable

```text
activos = pasivos + capital
```

Esta ecuación se cumple tras cada operación. Es la propiedad que la suite de tests aserta continuamente, y es lo que hace al contrato "contablemente sano": en cualquier punto puedes verificar que todo lo que el contrato posee está exactamente respaldado por algo que debe o que ha ganado.

## La estructura

```rust
pub struct Balance {
    pub assets:      BalanceAsset,
    pub liabilities: BalanceLiabilities,
    pub equity:      BalanceEquity,
}

pub struct BalanceAsset {
    pub payment_rigths:    i128,
    pub contract_earnings: i128,
    pub managed_volume:    i128,
    pub managed_collateral: i128,
}

pub struct BalanceLiabilities {
    pub payment_obligations:          i128,
    pub return_collateral_obligations: i128,
}

pub struct BalanceEquity {
    pub contract_commissions: i128,
}
```

## Qué significa cada campo

### Activos — lo que el contrato gestiona

| Campo | Significado |
|---|---|
| `managed_volume` | Valor total gestionado y adeudado a los inversores: el `total` de cada posición activa, más las transferencias de la empresa recibidas para rondas futuras. Disminuye a medida que se pagan a los inversores. |
| `contract_earnings` | Fondos de comisión en poder del contrato (lado de activos). Disminuye cuando se retiran las comisiones o se revierten mediante un reembolso. |
| `payment_rigths` | El contador de aportaciones de la empresa para rondas de pago, **almacenado con signo negativo**: se hace más negativo a medida que la empresa financia cada ronda, y vuelve hacia cero cuando se retiran fondos de proyecto. Mantiene equilibrada la ecuación de valor total entre reclassificaciones de custodia. |
| `managed_collateral` | Colateral en poder del contrato (en unidades del token de colateral). Disminuye cuando el colateral se liquida o se devuelve. |

> **Una nota sobre `payment_rigths`:** cuando la empresa envía una transferencia de `X` para una ronda de pago, el contrato recibe `X` tokens *y* reclasifica `X` de `managed_volume`. La entrada contable suma `X` a `managed_volume` y resta `X` de `payment_rigths`. Los activos netos no cambian (los tokens físicamente llegaron, y la obligación con la empresa queda ahora representada por un "derecho" negativo). Puedes verlo en vivo en el test `test_flow_with_reverse_loan`, que aserta `payment_rigths = -261, -522, -783, -1044` tras cada ronda de pago.

### Pasivos — lo que el contrato debe

| Campo | Significado |
|---|---|
| `payment_obligations` | Cantidad total aún debida a los inversores de todas las posiciones activas. |
| `return_collateral_obligations` | El colateral que el contrato debe devolver al proveedor — coincide con el colateral actualmente depositado. |

### Capital — lo que el integrador ha ganado

| Campo | Significado |
|---|---|
| `contract_commissions` | Comisión pendiente de retirar por el `manager`. Disminuye cuando se retiran comisiones, o se revierte por completo cuando una posición se reembolsa. |

## Cómo mueven los campos las operaciones

Cada función `recalculate_from_*` aplica de dos a cuatro movimientos de campos y re-verifica la ecuación:

| Operación | Campos tocados |
|---|---|
| `invest` (nueva posición) | `assets.contract_earnings += commission` · `equity.contract_commissions += commission` · `assets.managed_volume += total` · `liabilities.payment_obligations += total` |
| `process_investor_payment` | `assets.managed_volume -= amount` · `liabilities.payment_obligations -= amount` |
| `add_company_transfer` | `assets.payment_rigths -= amount` · `assets.managed_volume += amount` |
| `withdrawn` (empresa) | `assets.payment_rigths += amount` · `assets.managed_volume -= amount` |
| `withdrawn_commissions` | `assets.contract_earnings -= amount` · `equity.contract_commissions -= amount` |
| `add_collateral` | `assets.managed_collateral += amount` · `liabilities.return_collateral_obligations += amount` |
| `pay_with_collateral` | `assets.managed_collateral -= collateral` · `assets.managed_volume -= remaining` · `liabilities.return_collateral_obligations -= collateral` · `liabilities.payment_obligations -= remaining` |
| `return_collateral_to_company` | `assets.managed_collateral -= amount` · `liabilities.return_collateral_obligations -= amount` |
| `withdrawn_all_collateral` | `assets.managed_collateral -= amount` · `liabilities.return_collateral_obligations -= amount` (el colateral restante se envía a una dirección `manager`) |
| `refund_investor` | `assets.managed_volume -= total` · `assets.contract_earnings -= commission` · `liabilities.payment_obligations -= total` · `equity.contract_commissions -= commission` |
| `withdrawn_all` | todos los campos reseteados a cero |

## Ejemplo 
La siguiente tabla de ejemplo muestra los asientos contables correspondientes a los siguientes eventos:
- Una inversión de `500` unidades al `5%` (`i_rate = 500`) con comisión `2`:
- Un retiro de la compañía por valor de `300` unidades
- Una aportación de la compañia para pagos de `60` unidades
- Un pago efectuado de `60` unidades a un inversor

> Los valores son solo ejemplos para demostrar como se cumple la ecuación `activo = pasivo + PN`


| **Activo** | | | | **Pasivo** | | **Patrimonio Neto** |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `managed_volume` | `managed_collateral` | `payment_rights` | `contract_earnings` | `payment_obligations` | `return_collateral_obligations` | `contract_commissions` |
| 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 522 | 0 | 0 | 2 | 522 | 0 | 2 |
| 222 | 0 | 300 | 2 | 522 | 0 | 2 |
| 282 | 0 | 240 | 2 | 522 | 0 | 2 |
| 222 | 0 | 240 | 2 | 462 | 0 | 2 |

Descripción de los asientos contables uno a uno:

1. En un inicio todas las columnas del balance están a `0`
2. Al entrar una inversión de `500` unidades se genera una posición con plusvalía de `22` unidades y `2` unidades de comisión. Esto genera `524` unidades en la parte del activo: `522` como volumen negociado y `2` como ganancias del contrato en forma de comisión. Por la parte del pasivo, aumentan las obligaciones de pago a inversores en `522` unidades y, por último, el patrimonio neto aumenta en `2` unidades fruto de las comisiones generadas sobre la inversión recibida. Así pues: `522 + 2 = 522 + 2`
3. Cuando la empresa realiza un retiro de `300` unidades la parte del activo reduce su volumen negociado en `300` unidades pero, al mismo tiempo, los derechos de cobro (activo para el contrato y pasivo para la empresa) también aumentan en 300. La parte del pasivo y del PN quedan inalteradas dado que las obligaciones de pago con los inversores no varían ni tampoco el PN ya que no se han generado más comisiones. Así pues: `222 + 300 + 2 = 522 + 2`
4. Ahora la empresa transfiere `60` unidades de token para cubrir una ronda de pagos. En este caso, al igual que en el anterior, solo se ve alterada la columna del activo. El volumen negociado aumenta en esas `60` unidades (liquidez que vuelve a contrato) pero las obligaciones de pago de la empresa con el contrato (activo para el contrato) se reducen también en esas `60` unidades. Así pues: `282 + 240 + 2 = 522 + 2`
5. Por último, se usan esas `60` unidades recibidas para liquidar el pago sobre una posición. En este caso, el activo reduce su volumen negociado en `60` unidades pero las obligaciones de pago con los inversores (pasivo para el contrato) también se reducen en esas mismas unidades. Así pues: `222 + 240 + 2 = 462 + 2`

La ecuación sobrevive a cada movimiento. Si alguna parte del código con bugs actualizara el balance de forma incorrecta, `check_balance()` haría fallar la operación con `Error::BalanceEquationViolation`.

## Consultar el balance on-chain

La función `get_contract_balance` requiere que la dirección de origen sea el `admin` o posea cualquiera de los roles (`operator`, `company`, `manager`):

```bash
stellar contract invoke \
  --id <YOUR_CONTRACT_ID> \
  -s admin \
  --network testnet \
  -- \
  get_contract_balance \
  --caller "G...."
```

La respuesta es el snapshot completo de `Balance`, agrupado por `assets`, `liabilities` y `equity`.

## El evento BalanceUpdated

Las operaciones que modifican el balance emiten el evento `BalanceUpdated` para que integradores e indexadores puedan seguir la contabilidad off-chain:

```rust
BalanceUpdated {
    assets: BalanceAsset,
    liabilities: BalanceLiabilities,
    equity: BalanceEquity,
}
```
