---
sidebar_position: 14
title: Apéndice de códigos de error
---

# Apéndice: códigos de error de negocio

Todos los errores de negocio se declaran en `src/shared/types.rs` como el enum canónico `Error`. Los valores son discriminantes numéricos estables — los integradores pueden apoyarse en ellos para su propio mapeo de errores. El enum no contiene códigos huérfanos: toda variante declarada se devuelve en algún path del contrato. Los códigos 52 (`EmptyEmergencyPool`), 55 (`PaymentsObligationsRemaining`) y 60 (`RetuenCollateralObligationsRemaining`) fueron eliminados del enum al quedar sin uso tras la nueva semántica de `withdrawn_all`.

| Código | Nombre | Cuándo se dispara |
|---:|---|---|
| 1 | `AddressInsufficientBalance` | El inversor tiene menos que el importe de la inversión, o el proveedor de colateral menos que el depósito. |
| 2 | `ContractReserveInsufficientBalance` | Un importe de pago excede el balance del token del contrato, o la transferencia de empresa dejaría el fondeo proyectado por debajo de `amount_to_pay_per_month`. |
| 3 | `ContractInsufficientBalance` | El contrato carece de liquidez para pagar un reembolso/pago/retirada. |
| 5 | `AmountLessThanMinimum` | Inversión por debajo de `min_per_investment`. |
| 6 | `InterestRateMustBeGreaterThanZero` | Validación del constructor. |
| 7 | `GoalMustBeGreaterThanZero` | Validación del constructor. |
| 8 | `UnsupportedReturnType` | Constructor: `return_type` debe ser `1` (ReverseLoan) o `2` (Coupon). |
| 9 | `ReturnMonthsMustBeGreaterThanZero` | Validación del constructor. |
| 10 | `MinPerInvestmentMustBeGreaterThanZero` | Validación del constructor. |
| 11 | `CmrUpperDividorMustBeGreaterThanZero` | Validación del constructor. |
| 12 | `CmrLowerDividorMustBeGreaterThanZero` | Validación del constructor. |
| 13 | `CmrReductorMustBeGreaterThanZero` | Validación del constructor. |
| 14 | `AddressHasNotInvested` | El `position_id` referenciado no existe (o falta el mapeo id→dirección). |
| 15 | `CmrUpperDivisorMustBeGreaterTheCmrLowerDivisor` | Validación del constructor (`cmr_upper_divisor` debe ser mayor que `cmr_lower_divisor`). |
| 16 | `AddressAlreadyHoldsRole` | Grant de un rol a una dirección que ya lo posee. |
| 17 | `AddressRoleAlreadyExists` | Grant de `company` o `manager` mientras ese rol ya tiene un miembro (regla de titular único). |
| 28 | `RecipientCannotReceivePayment` | La transferencia de tokens fue rechazada por el contrato del token (p. ej. el contrato receptor no puede confiarse para aceptar). |
| 29 | `InvalidPaymentData` | La transferencia de tokens devolvió un resultado inválido. |
| 30 | `GoalReached` | `invest` tras alcanzar el objetivo de fundraising. |
| 33 | `CollateralLevelNotComputable` | El nivel de colateral no pudo computarse (pricing del oráculo no disponible) en `add_collateral`. |
| 34 | `OnlyOneCollateralTokenAllowed` | Colateral ya configurado con un token distinto. |
| 36 | `CollateralNotConfigured` | No hay colateral configurado para esta instancia. |
| 37 | `CollateralBalanceIsEmpty` | `return_collateral_to_company` sin nada en poder. |
| 38 | `PositionCompleted` | Operación sobre una posición completada (pago, reembolso, liquidación de colateral). |
| 39 | `FundrasingTimeOngoingYet` | Operación que exige que el periodo de fundraising haya terminado (retiradas, activación de emergencia). |
| 40 | `FundrasingTimeExceeded` | `invest` tras el cierre de la ventana de fundraising. |
| 42 | `PaymentAlreadyProcessedForThisPeriod` | El contador de ronda de la posición no coincide con la ronda de pago actual. |
| 43 | `NextPaymentCannotBeScheduledYet` | Transferencia de empresa antes de `ts_payments_start`. |
| 45 | `EmptyPaymentObligations` | Guard contable en `pay_with_collateral`: las obligaciones totales de pago no son positivas (estado contable degenerado). |
| 46 | `EmptyRefundAmount` | Reembolso de importe cero (guard defensivo). |
| 47 | `OwnerInsufficientBalance` | El origen de la `company` carece de fondos para una transferencia. |
| 48 | `EmergencyAlreadyActive` | Activación de emergencia estando ya activa. |
| 49 | `EmergencyNotActive` | `withdrawn_all` o `withdrawn_all_collateral` sin la compuerta de emergencia activa. |
| 50 | `OperationNotAllowedInEmergency` | Cualquier operación que mueve capital durante el modo de emergencia. |
| 51 | `PendingCommissionWithdrawal` | Activación de emergencia con comisiones aún pendientes de retirar. |
| 53 | `PositionIdAlreadyExists` | El `position_id` estaba ya usado. |
| 54 | `LiquidationPaymentsOutOfPeriod` | Enable/disable de liquidaciones fuera de la ventana fin-de-fundraising → inicio-de-pagos. |
| 56 | `BalanceUpdateOverflow` | Desbordamiento de suma checked durante un recálculo del `Balance`. |
| 57 | `BalanceUpdateUnderflow` | Subdesbordamiento de resta checked durante un recálculo del `Balance`. |
| 58 | `CollateralAmountOverflow` | El colateral total desbordaría `i128`. |
| 59 | `CollateralPriceOracleError` | El cross price no pudo computarse durante la liquidación (sin precio de oráculo o precio cero del token del contrato). |
| 61 | `TooLargeInvestment` | Depósito demasiado grande para la aritmética WAD — rechazo limpio no-op: nada se transfiere ni almacena. |
| 62 | `BalanceEquationViolation` | La verificación de la ecuación contable falló durante un recálculo del `Balance` — solo alcanzable mediante un bug contable interno. |
| 63 | `EmptyCommissionBalance` | Retirada de comisiones sin comisión pendiente. |

## Modelo de propagación de errores

- **Errores de validación** se disparan *antes* de cualquier transferencia de tokens o cambio de estado, así que una llamada rechazada es siempre un `no-op` completo.
- **Errores de transferencia** (`RecipientCannotReceivePayment`, `InvalidPaymentData`) mapean fallos del contrato del token a códigos de negocio estables.
- **Errores contables** (`BalanceUpdateOverflow`/`Underflow`/`BalanceEquationViolation`) se disparan *antes* de persistir un balance roto. La ecuación `activos = pasivos + capital` se re-verifica dentro de cada recálculo, así que una contabilidad inconsistente nunca puede persistirse.
- Los rangos de código conocidos se testean uno a uno en `error_tests.rs` (ver [Tests](./tests.md)).
