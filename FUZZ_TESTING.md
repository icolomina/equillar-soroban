# Fuzz Testing por Propiedades — Guía del contrato `investment-income-based`

> Documento de referencia para la estrategia de fuzz testing del contrato.
> Fase actual: **Fase 1 (Suite A) + Fase 2 (Suite B FSM)** — propiedades de la
> matemática de asignación (`allocation.rs`) y secuencias aleatorias de
> operaciones contra un modelo de referencia (`tests/common/fuzz_model.rs`,
> `tests/fuzz_fsm.rs`).
> Si este fichero se mueve, actualizar la ruta en este encabezado.

---

## 1. Qué es el fuzz testing por propiedades (y por qué se usa aquí)

Un test unitario tradicional dice: *"con esta entrada concreta, espero esta
salida concreta"*. Un **test de propiedades** dice lo contrario:

> *"Sin importar qué entrada válida generes aleatoriamente, esta regla debe
> cumplirse siempre."*

Esa regla se llama **invariante** (o *propiedad*). El framework (aquí,
[`proptest`](https://docs.rs/proptest)) hace tres cosas:

1. **Genera** cientos de entradas aleatorias dentro de un dominio definido
   (la *estrategia*).
2. **Ejecuta** la propiedad con cada entrada. Si la propiedad se cumple,
   pasa a la siguiente.
3. **Minimiza** (*shrinking*) cuando una entrada rompe la propiedad:
   reduce el contraejemplo a la versión más pequeña que sigue fallando.
   Un contraejemplo mínimo es casi siempre un bug obvio y diagnosticable.

Un contraejemplo es una **entrada que viola una regla que nunca debería
violarse**. O es un bug del contrato, o es una propiedad mal formulada — y
distinguir entre ambas es el trabajo de triage (sección 5).

Por qué encaja bien con Soroban: los contratos se pueden compilar a *native*
para tests, así que el ciclo generar → llamar → asertar corre a velocidad de
Rust, sin node ni simulaciones pesadas.

---

## 2. Diferencia con los tests que ya tenemos

| Aspecto | `success_tests.rs` / `error_tests.rs` | `fuzz_properties.rs` (Suite A) |
|---|---|---|
| Pregunta que responde | "¿Este caso concreto se comporta como documentamos?" | "¿Existe *algún* caso en el que la regla se rompa?" |
| Entradas | Fijas, escritas a mano | Aleatorias, cientos por ejecución |
| Cobertura implícita | La que el autor imaginó | Bordes numéricos, combinaciones de parámetros que nadie pensó |
| Fallo esperado | Nunca (si pasa, está bien) | **Esperable ocasionalmente**: un fallo es un hallazgo, no una molestia |
| Mantenimiento | Se adapta al cambiar la lógica | Se adapta al cambiar las *invariantes de negocio* |
| Reproducibilidad | Total | Total también: seeds y contraejemplos se persisten |
| Ejemplo de hallazgo | — | **W-2** (sección 6): un overflow real encontrado en segundos |

Los tests existentes siguen siendo la primera línea de defensa (regresión de
comportamiento documentado). La Suite A ataca lo que los tests unitarios no
pueden: *combinaciones de entradas no previstas*.

---

## 3. Qué testeamos en la Fase 1: las propiedades

Objetivo: la matemática de asignación que se ejecuta al invertir
(`create_position` → `calculate_rate_denominator` + `get_deposit_allocation` +
`calculate_regular_payment`). Ese código es puro, determinista y **crítico**:
define cuánto se cobra de comisión, cuánto se abona y cómo se reparte el pago
mensual. Un error aquí corrompe la contabilidad de todo el contrato.

Al no poder llamarse directamente (los módulos son privados), las propiedades
se ejercitan **a través del entrypoint público `invest`**, que devuelve la
`Position` resultante. Así además el fuzzing valida el camino completo
(validaciones + transfer + contabilidad), no solo la función.

Propiedades codificadas (invariantes P del plan de seguridad):

| ID | Propiedad | Por qué importa |
|---|---|---|
| **P1** | `deposited + commission == amount` | El dinero del inversor se reparte **exactamente**; no se pierde ni se crea dust en la entrada |
| **P2** | `total == deposited + returns` | El total reclamable es exactamente principal + rendimientos |
| **P3** | `commission ≤ amount·i_rate / (den·10_000)` | La comisión nunca excede el floor de la tasa del tier correspondiente (el test recalcula el `denominator` con la misma aritmética entera que `calculate_rate_denominator`) |
| **P4** | `returns ≤ deposited·i_rate / 10_000` | Los rendimientos no pueden exceder el `i_rate`% del principal invertido |
| **P5** | `regular_payment == base / months` (división entera), `regular_payment·months ≤ base` y el pago final (`base − regular·(months−1)`) ≥ `regular_payment` | El calendario de pagos siempre cierra dentro de `base` (para ReverseLoan `base = total`; para Coupon `base = returns`) |
| **P8** | Para dos inversiones del **mismo tier** con `a ≤ b`: `commission`, `deposited`, `returns`, `total` y `regular_payment` son monotónicos no decrecientes | Más dinero invertido ⇒ salida mayor o igual, nunca invertida (ver sección 6.2 para por qué solo *dentro del tier*) |
| **INV-1** | Tras cada inversión: `Σ assets == Σ liabilities + Σ equity` | La ecuación contable de doble entrada, verificada con el helper `assert_balance_equation` de `tests/common/mod.rs` |

Cada caso de fuzz despliega un contrato real (con el mock SAC de token y el
mock de oracle `ReflectorMock`), hace un `mint` y una inversión real. No se
mockea la lógica: se fuzzea el contrato de verdad.

---

## 4. Anatomía del código generado

### 4.1 Estructura de ficheros

```
contracts/investment-income-based/
├── Cargo.toml                      ← proptest añadido como dev-dependency
└── tests/
    ├── common/mod.rs               ← setup compartido (ya existía)
    ├── fuzz_properties.rs          ← Suite A (Fase 1, nueva)
    ├── fuzz_properties.proptest-regressions  ← seeds de fallos (auto-generado)
    ├── success_tests.rs            ← tests unitarios de comportamiento
    └── error_tests.rs              ← tests unitarios de errores
```

### 4.2 Crates y módulos que intervienen

| Crate / módulo | Papel en el test |
|---|---|
| `proptest` (dev-dependency) | Generación aleatoria + shrinking + reproducibilidad |
| `soroban-sdk` (con feature `testutils`) | `Env` determinista de test, `Address::generate`, mocks de auth |
| `investment_income_based::contract::InvestmentContractClient` | Cliente generado por la macro `#[contractimpl]`; los métodos (`invest`, `get_contract_balance`, ...) son las llamadas reales al contrato desplegado |
| `tests/common/mod.rs` | `create_investment_contract` (despliegue completo con mocks), `invest_as_operator` (grant de rol + invest), `assert_balance_equation` |
| `investment_income_based::shared::types::Position` | El resultado de `invest`; sus campos son lo que se aserta |

### 4.3 Las *estrategias* (el corazón de la generación)

Una estrategia describe **qué entradas se generan y de qué rango**. En el
fichero hay cuatro piezas que conviene entender porque son el patrón a seguir
en las siguientes fases:

**a) Estrategias simples** — rangos directos:

```rust
fn arb_i_rate() -> impl Strategy<Value = u32> {
    1u32..=10_000                       // 1 bp .. 10_000 bp (100 %)
}
```

**b) Filtro con `prop_filter`** — dominios con condiciones entre campos:

```rust
fn arb_cmr_pair() -> impl Strategy<Value = (u32, u32)> {
    (1u32..=1_000, 1u32..=10_000)
        .prop_filter("cmr_upper must exceed cmr_lower", |(lower, upper)| upper > lower)
}
```

Genera el par y **descarta** los que no cumplan. (Útil cuando el ratio de
descarte es bajo; si fuera alto, ver c).

**c) Generación dependiente** — cuando un campo depende de otro. Es la parte
más sutil del código. Necesaria porque el hallazgo W-2 (sección 6.1) impone
la restricción `amount × i_rate < 1.7e13`:

```rust
fn arb_invest() -> impl Strategy<Value = (u32, i128)> {
    arb_i_rate()
        .prop_flat_map(|i_rate| (1i128..=(SAFE_AMOUNT_RATE / i_rate as i128), Just(i_rate)))
        //      ^ el rango de `amount` DEPENDE del `i_rate` ya generado
        .prop_map(|(amount, i_rate)| (i_rate, amount))
}
```

La cadena de composición estándar de proptest:

- `prop_flat_map`: genera `i_rate`, y con él genera un nuevo valor cuyo
  dominio usa `i_rate`. (`Just(x)` es la estrategia que devuelve siempre `x` —
  se usa para "pasar hacia abajo" un valor ya generado).
- `prop_map`: transforma el resultado final (reordena la tupla).

Para pares en el **mismo tier** (`arb_same_tier_pair`) la dependencia es de
dos niveles: primero `i_rate`+`cmr`+`reductor`, luego `a`, luego `b` acotado
para **no cruzar el siguiente escalón** de `reductor`:

```rust
// `b` no puede pasar de (a/reductor + 1)·reductor − 1: si lo cruzara,
// el `denominator` cambiaría y la comparación dejaría ser justa.
let tier_end = (a / reductor + 1) * reductor - 1;
let b_max = tier_end.min(SAFE_AMOUNT_RATE / i_rate as i128);
(a..=b_max, Just((a, i_rate, cmr, reductor)))
```

**d) Config del runner:**

```rust
#![proptest_config(proptest::test_runner::Config {
    cases: 128,          // casos por propiedad (ampliable, ver abajo)
    max_shrink_iters: 5_000,
    ..Config::default()
})]
```

Escalar el número de casos **sin tocar el código**:

```bash
PROPTEST_CASES=10000 cargo test --test fuzz_properties   # exploración agresiva local
```

### 4.4 Las aserciones dentro de `proptest!`

`prop_assert!` / `prop_assert_eq!` son como `assert!`/`assert_eq!`, pero al
fallar capturan el valor de las entradas generadas y disparan el *shrinking*.
El mensaje (`"commission {} exceeds tiered-rate floor (den={denominator})"`)
es deliberadamente informativo: al triar un contraejemplo, ese texto es la
primera pista.

### 4.5 Reproducibilidad

- Cada `Env::default()` de soroban es determinista; con la misma semilla,
  el mismo caso genera exactamente el mismo mundo.
- Cuando una propiedad falla, proptest **persiste la semilla** del caso en
  `tests/fuzz_properties.proptest-regressions`:

  ```text
  cc ae15ef37b4e8d00c91bbbf7453b21257fb09e35be0b92a761f4604f98c2fcbd9
      # shrinks to i_rate = 1, cmr = (1, 2), ..., amount = 17014969094502
  ```

  Ese fichero se re-ejecuta **antes** de generar casos nuevos en cualquier
  run futuro (incluido CI). **Se debe commitear**: es el equivalente a los
  `test_snapshots`, pero para contraejemplos. Nunca editar a mano.

---

## 5. Cómo se encuentran y resuelven los hallazgos

Ciclo de vida completo de un hallazgo:

```
fallo de propiedad
   → contraejemplo minimizado (lo imprime el test y lo persiste)
   → reproducirlo aislado (se re-ejecuta solo con la seed)
   → triage: ¿bug del contrato, propiedad mal formulada, o dominio acotado?
   → si bug: fix en el contrato + test de regresión dirigido en error_tests.rs
   → si propiedad inválida: reformular (documentando por qué)
   → la semilla del fichero de regresiones pasa a pasar (queda como vigilancia)
```

Tres categorías de triage, con ejemplos reales de esta fase:

### 5.1 Bug real del contrato → fix + regresión dirigida

Se reconoce porque la violación contradice una regla de negocio válida y el
contraejemplo no depende de ningún comportamiento raro. Solución típica:
corregir el contrato y convertir el contraejemplo en un `#[should_panic]`
(espera el nuevo error limpio) o en un test de éxito en `success_tests.rs`.

### 5.2 Propiedad mal formulada → reformular

Se reconoce porque la "violación" es semántica de negocio deliberada.
Ejemplo real de esta fase (**W-3**): la propiedad P8 original exigía
monotonicidad de la comisión **cruzando tiers**, y el fuzzing produjo:

```text
i_rate=5616, cmr=(1,2278), reductor=492112,
small=1121418804, big=1119554039
```

Con esos valores, invertir `small` cae en tier `den=2275` y `big` (mayor) en
tier `den=2279` → tasa menor → comisión total menor. Invertir más y pagar
menos comisión total **es el diseño del sistema de tiers** (el rate baja
explícitamente con el importe). La propiedad se reformuló a
`allocation_monotonic_within_tier` (misma comparación, garantizando que
ninguna de las dos inversiones cruce un escalón), y el documento del test
explica por qué la monotonicidad entre tiers **no** es una invariante.

### 5.3 Dominio a acotar (panic conocido) → bound + hallazgo documentado

Se reconoce porque la "violación" es un **trap del host** (p. ej.
`Error(WasmVm, InvalidAction)` con `caught panic 'attempt to multiply with
overflow'`), no un `Error(Contract, #N)` limpio. El contrato *no puede
manejar* ese dominio hoy. Dos pasos: (1) acotar la estrategia fuera del
dominio de pánico para que la suite sea verde y siga explorando el dominio
sano; (2) registrar el boundary como hallazgo con test dirigido en la Fase 4.

---

## 6. Hallazgos producidos por la Fase 1

### 6.1 W-2 — Overflow (trap) en `invest` con importes grandes · **[ALTA]**

**Síntoma.** `invest` aborta con `Error(WasmVm, InvalidAction)` y log
`caught panic 'attempt to multiply with overflow'` en lugar de devolver un
`Error` de negocio limpio.

**Causa raíz.** El operador `Mul` de `Wad` (crate `stellar-contract-utils`,
`packages/contract-utils/src/math/wad.rs:789`) hace el producto raw **sin
checked**:

```rust
fn mul(self, rhs: Wad) -> Wad {
    Wad((self.0 * rhs.0) / WAD_SCALE)   // ← pánico si self.0 * rhs.0 deja i128
}
```

(a diferencia de `Wad::checked_mul`, que sí tiene fallback a I256).
En `get_deposit_allocation`, el cálculo `amount_wad * return_rate_wad`
desborda i128 cuando `amount × i_rate ≥ i128::MAX / 10^25 ≈ 1.7e13`
(token de 7 decimals). Con `i_rate = 10000 bp` eso significa no poder
invertir ≥ ~1.7M tokens — importes perfectamente realistas para un goal de
primos grandes.

**Contraejemplo mínimo (shrinkado por proptest):**

```text
i_rate = 1, cmr = (1, 2), cmr_reductor = 1,
return_type = 1 (ReverseLoan), return_months = 1,
amount = 17014969094502, token_id = 0
```

(El amount está afinado con precisión de 1 unidad al boundary del overflow —
por eso el shrink es tan útil: la frontera exacta, no un caso grande al azar.)

**Fix propuesto.** Sustituir los operadores `*` de `Wad` en
`get_deposit_allocation` por `Wad::checked_mul` (que usa `checked_mul_div`
con fallback I256) y mapear `None` a un error de negocio
(`Result<Position, Error>`). La Suite A, con el bound relajado, sería la
regresión automática del fix.

**Estado.** Estrategias acotadas a `amount × i_rate < 1.7e13`; test dirigido
del boundary pendiente para la Fase 4.

### 6.2 W-3 — Monotonicidad entre tiers no es invariante · **[NO BUG]**

Descrito en 5.2: reformulación de P8, documentada en el propio test.

---

## 7. Cómo operar la suite

```bash
# Suite A sola (lo que hay en Fase 1)
cargo test --test fuzz_properties

# Suite completa del contrato
cargo test

# Exploración agresiva (desarrollo de un fix, pre-release)
PROPTEST_CASES=10000 cargo test --test fuzz_properties

# Reproducir un contraejemplo concreto: la seed se re-ejecuta sola;
# para iterar rápido sobre ella:
cargo test --test fuzz_properties allocation_single
```

Notas operativas:

- Las propiedades viven en el binario de tests normal de `cargo test`; no
  requieren toolchain nightly ni infra especial (eso será el Tier 2 con
  cargo-fuzz, fuera del pipeline).
- Si cambias la lógica de `allocation.rs`, las propiedades no necesitan
  cambios (solo si cambian las **reglas de negocio**). Si la suite rompe tras
  un cambio legítimo, significa que el cambio viola una invariante
  documentada → decidir: fix del contrato o reformulación justificada.
- El coste añadido al `cargo test` diario es ~13 s (128 casos × 2
  propiedades, cada caso despliega contrato + SAC + oracle mock).

## 8. Roadmap del fuzzing (contexto de este documento)

| Fase | Contenido | Estado |
|---|---|---|
| 0 | fmt + inventario de tests afectados por los cambios del enum `Error` | ✅ |
| **1 — Suite A** | Propiedades puras de `allocation.rs` (este documento) | ✅ |
| **2 — Suite B** | Modelo de referencia (`tests/common/fuzz_model.rs`) + FSM de secuencias de ops (`tests/fuzz_fsm.rs`) | ✅ |
| 3 — Suite C | Tests dirigidos de regresión de las correcciones (refund/comisión, `withdrawn_all` + colateral, INV-21) | pendiente |
| 4 | Fuzzing agresivo + triage de hallazgos (W-1..W-5: W-2/W-4/W-5 boundary, A3 donaciones, A4 oracle, A5 divisiones por cero) | pendiente |
| 5 | Ajuste del typo #60, decisión INV-6b, adaptación de tests existentes, crate `fuzz/` (cargo-fuzz, uso manual, fuera del pipeline) | pendiente |

---

## 9. Fase 2 — modelo de referencia y FSM de secuencias (Suite B)

La Suite A valida la matemática de una sola inversión. La Suite B ataca el
problema de seguridad real: **combinaciones de operaciones que nadie prueba**.
Un caso de fuzz es una cadena de 0..=30 operaciones (`Op`) sobre un contrato
real, y tras CADA operación se verifican todas las invariantes.

### 9.1 Arquitectura: `tests/common/fuzz_model.rs`

- **`Op`** — un enum con las ~17 operaciones que mueven estado: inversión,
  pagos, refunds, tesorería (`withdrawn*`), colateral, emergencia, pausa,
  liquidaciones, más la **superficie de intruso**: `DonateContractToken`,
  `DonateCollateralToken` (transferencias SAC directas al contrato, vector
  A3) y `SetOraclePrice` (el mock del oracle acepta reescribir precios,
  vector A4), y `AdvanceTime` (el ledger avanza días aleatorios 0..=40).
- **`Model`** — espejo de la contabilidad de doble entrada en i128 puro,
  incluyendo la aritmética WAD exacta (todas las divisiones truncantes) de
  asignación, nivel de colateral y liquidación proporcional. También espejo
  la liquidez real (saldo SAC del contrato, donativos incluidos).
- **El driver es reactivo**: nunca predice Ok/Err. Éxito ⇒ aplica la
  semántica esperada al espejo y compara campo a campo con la cadena;
  fracaso ⇒ el estado on-chain debe estar *bit a bit* intacto.

### 9.2 Invariantes verificadas en cada paso

| ID | Qué |
|---|---|
| **Diferencial** | El `Balance` on-chain == espejo del modelo, campo a campo, tras cada op |
| **INV-1** | Ecuación contable `assets == liabilities + equity` (en cadena y en el modelo) |
| **INV-18** | Una op de negocio que termina en *trap* del host en vez de error de contrato = hallazgo (el test falla y proptest minimiza) |
| **INV-19** | Atomicidad: tras cualquier fracaso, contabilidad + saldo real + posición objetivo intactos |
| **INV-liq** | La liquidez real siempre es igual al espejo (detecta inflows no rastreados y rollbacks incompletos) |

Los errores se clasifican con la forma exacta de los `try_*` del SDK:
`Ok(Ok(v))` éxito; `Err(Ok(e))` o `Err(Err(InvokeError::Contract(_)))` error
de contrato (el nuestro o uno foráneo, p. ej. `PausableError`) ⇒ solo se
comprueba atomicidad; `Err(Err(InvokeError::Abort))` ⇒ trap ⇒ INV-18.

### 9.3 Dominios acotados (y por qué)

Los límites existen para que la suite explore el *dominio de negocio* y no
re-descubra los traps conocidos (los boundaries son hallazgos, ver 9.4):
- `invest ≤ 30_000_000` unidades por op: mantiene ≥5× de margen en todos los
  productos WAD (W-2) y po total ≤ 1.35e9 vs. el límite de 1.7e9 de W-5.
- Presupuesto total de colateral 20_000 unidades (depósitos + donativos
  combinados) por W-4; ops que agotarían el presupuesto se omiten.

### 9.4 Hallazgos producidos por la Fase 2

**W-4 — Traps en `add_collateral`/`pay_with_collateral` con colateral realista
· [ALTA, DoS].** `calculate_collateral_level` y
`get_collateral_for_investment` multiplican `collateral_wad * cross_price`
sin *checked*. Con un precio oracle realista (60_000 de un activo ~$60k en
escala 8-dec frente al token ~$1), `collateral_value_wad = col·1e11 · 6e22`
sale de i128 a partir de **~28_300 unidades crudas de colateral** (~0.003
tokens): la funcionalidad de colateral trap'a con depósitos minúsculos y
queda bloqueada. Esta misma FSM lo encontró como "attempt to multiply with
overflow" en la primera corrida; la suite se acota bajo el boundary
(`MAX_COLL_CAP`) y el triple exacto queda para el test dirigido de Fase 4.
Fix propuesto: `Wad::checked_*`/fallback I256 (misma familia que W-2), o
re-escalar la contabilidad del colateral.

**W-5 — `pay_with_collateral` trapea cuando las obligaciones de pago ≥ ~170
tokens · [ALTA, DoS parcial].** `Wad::Div` calcula
`(po_wad · WAD_SCALE) / cross`; el numerador `po·1e11 · 1e18` sale de i128
cuando `payment_obligations ≥ 1.7e9` unidades (170 tokens con 7 decimales).
Dado que la rama del Div se recorre casi siempre (`cv > po` con precios
realistas), **una inversión de ~165 tokens hace imposible liquidar** esa
posición vía colateral hasta que las obligaciones bajen de nuevo. La suite
se acota a po ≤ 1.35e9; el boundary dirigido queda para Fase 4.

**Detalle de salvaguardas encontradas por el fuzzing (no bugs):** refund de
una posición parcialmente pagada falla limpiamente (underflow de
`payment_obligations` ⇒ rollback atómico, INV-19 se mantiene), y
`pay_with_collateral` tras un refund total falla con
`PositionCompleted`/`CollateralNotConfigured` (la división por cero
analizada como A5 solo sería accesible con posiciones activas y po == 0,
que la contabilidad impide; se mantiene en la lista de Fase 4 por si el
fuzz agresivo lo contradice).

### 9.5 Operación

```bash
cargo test --test fuzz_fsm                                  # Suite B sola
PROPTEST_CASES=512 cargo test --test fuzz_fsm               # explotación
```

Coste: ~20 s por 128 casos (cada caso despliega contrato + 2 tokens SAC +
oracle mock y ejecuta hasta 30 ops con sus verificaciones). Las semillas de
fallas viven en `tests/fuzz_fsm.proptest-regressions` (commitear, no editar).
