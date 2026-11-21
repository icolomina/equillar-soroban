---
sidebar_position: 13
title: Tests y cobertura
---

# Tests y cobertura de testing

La suite de tests vive en `contracts/investment-income-based/tests/` y está dividida en tres tipos, cada uno con un objetivo de protección distinto. Ejecutar `cargo test` desde la raíz del repo los ejecuta todos.

```text
tests/
├── common/mod.rs        # setup compartido (despliegue del contrato, mock de oráculo, token, helpers)
├── success_tests.rs     # 21 tests dirigidos de flujo extremo a extremo
├── error_tests.rs       # 44 tests dirigidos de guards
└── fuzz_properties.rs   # 2 tests de fuzzing basado en propiedades (proptest)
```

## 1. `success_tests.rs` — valores exactos del ciclo de vida (21 tests)

Flujos completos extremo a extremo con aserciones **exactas, a nivel de campo**, tanto del `Position` devuelto como del snapshot `Balance` tras cada paso. Fijan el comportamiento de truncamiento/redondeo bit a bit y cubren:

- Ciclos de vida completos Coupon y ReverseLoan (todas las rondas, devolución del capital final, completado)
- Rondas de pago con liquidaciones **deshabilitadas** (la contabilidad se actualiza sin transferencias dentro del contrato)
- Múltiples inversores en la misma ronda de pago
- Retiradas de empresa, retiradas de comisiones, barridos finales en emergencia (`withdrawn_all`, `withdrawn_all_collateral`) etc
- Flujos de depósito/liquidación/devolución de colateral
- Pausa/reanudación, revocación de operator bloqueando `invest`, flujos de grant
- La ecuación de partida doble (`activos = pasivos + capital`) testeada después de cada operación

Los ejemplos trabajados de esta documentación ([Fundraising](./fundraising-and-positions.md#ejemplo-a-escala-del-token)) reproducen estos valores exactos, así que cada cifra que lees aquí es verdad ejecutable.

## 2. `error_tests.rs` — cada guard, un test por `callsite` (44 tests)

Un test por guard de error de negocio × entrypoint, asertando tanto el error de negocio limpio (sin trap) como el código de panic esperado. Cobertura:

- Validación del constructor (tasa de interés, goal, meses, mínimo, parámetros cmr, tipo de retorno)
- Guards de `invest` (balance, mínimo, ventana, goal alcanzado, `token_id` duplicado, emergencia)
- Guards de rondas de pago (sin reserva, ronda incorrecta, enable/disable fuera de periodo)
- Guards de reembolso (ya reembolsado, fundraising terminado, nunca invirtió)
- Guards de retirada (sin comisión disponible, ya retirada, fundraising en curso)
- Guards de colateral (no configurado, vacío, solo un token)
- Guards de emergencia (ya activa, comisión pendiente, operaciones bloqueadas durante emergencia)
- Guards de roles (ya posee el rol, el rol ya existe, regla single-holder de company/manager)

## 3. `fuzz_properties.rs` — fuzzing basado en propiedades de las asignaciones (2 propiedades)

Esta suite realiza **fuzzing de la matemática de asignación a través del entrypoint público `invest`** usando [proptest](https://proptest-rs.github.io/proptest/). Explora importes desde 1 unidad raw hasta `10^27` y tasas de interés de 1 a 10.000 puntos básicos sobre combinaciones aleatorias de parámetros de despliegue, y aserta, para cada caso:

**En éxito (`Ok`):**
- **P1** — `deposited = amount - commission`
- **P2** — la comisión pertenece al tramo actual: `0 ≤ commission ≤ floor(amount × i_rate / (denominator × 10_000))` con el tramo resuelto exactamente como se documenta
- **P3** — `returns = floor(deposited × i_rate/10_000)` (tope independiente del tramo)
- **P4** — `total = deposited + returns`
- **P5** — `regular_payment` coincide con la cuota correcta del schedule (Coupon: `returns / return_months`; ReverseLoan: `total / return_months`)
- **INV-1** — la ecuación de partida doble cumple: `activos = pasivos + capital`
- el rechazo no-op es válido: un `Err(TooLargeInvestment)` limpio se aserta como **no-op completo** (balance del inversor intacto, balance del contrato sin cambios, todos los campos del Balance sin cambios)

**Además:**
- **P8** — monotonicidad de la comisión intra-tramo: para un límite de tramo fijo, la comisión nunca decrece cuando el importe crece dentro del tramo (documenta semántica de negocio intencional)

Cualquier cosa que **no** sea uno de los dos resultados válidos: un trap, un `InvokeError::Abort` o cualquier error de negocio inesperado, hace fallar el test.

### Ejecución y escalado

```bash
cd contracts/investment-income-based
cargo test --test fuzz_properties            # 128 casos aleatorios por propiedad
PROPTEST_CASES=10000 cargo test --test fuzz_properties   # validación local más pesada
```

La integración continua ejecuta `cargo test` con los 128 casos por propiedad por defecto en cada push/PR a `main`.

### Los ficheros de semillas de regresión

Cuando una propiedad falla, proptest persiste una semilla de reductor (shrinking) en `tests/*.proptest-regressions` y la re-reproduce automáticamente en ejecuciones posteriores. 

> **Matiz importante**: las semillas codifican el estado del RNG bajo la **estrategia actual**, así que tras un fix de bug o un re-diseño de la estrategia ya no reproducen su contraejemplo original; borra los ficheros de semillas caducadas deliberadamente (y anótalo en el commit). Nunca edites a mano un fichero de semillas válido.

## Qué no cubre la suite (todavía)

Ser explícito con los límites de la protección actual:

- Los tests dirigidos ejercitan una configuración de token fija (7 decimales, mock SAC) y un mock de oráculo con precios hardcoded — divergencias de token cruzado o decimales exóticos solo se muestrean con la suite de fuzz en-wad.
- La suite de fuzz conduce `invest` de extremo a extremo, así que los estados alcanzables **después** de la aceptación (bordes temporales de rondas de pago, entrelazados multi-ronda de colateral) están cubiertos por tests dirigidos pero no fuzzeados.
- No hay simulación de churn del ledger / restauración TTL.

## Integración continua

El pipeline de CI (`.github/workflows/rust.yml`) ejecuta `cargo fmt --check`, `cargo clippy`, `cargo build` y el `cargo test` completo (los tres binarios, incluida la suite de fuzz con su recuento de casos por defecto).
