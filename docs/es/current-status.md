---
sidebar_position: 2
title: Estado actual
---

# Estado actual

Equillar debe entenderse como una **base técnica real con carencias visibles aún por cerrar** antes de un uso en producción.

## Resumen de estado

- **Contrato Soroban**: implementado y testado
- **Auditoría**: no completada
- **Objetivo de despliegue**: solo testnet — las funciones están siendo testadas ahí
- **Preparación para producción**: no lista

## Qué existe hoy

El protocolo actual ya incluye:

- Configuración a nivel de constructor para las condiciones de financiación, con validación completa de parámetros en el despliegue
- Gestión de roles para `admin`, `operator`, `company` y `manager`
- Entrada de inversiones y gestión de posiciones, con aritmética de asignación checked (sin traps) y un rechazo limpio con `TooLargeInvestment` fuera del rango representable
- Contabilidad de partida doble (`Balance`), verificada tras cada actualización contable
- Reembolsos durante el fundraising, con reversión completa de la comisión
- Rondas de pago financiadas por las transferencias de la empresa, además de un modo de pago fuera del contrato
- Retiradas de empresa y de comisiones
- Soporte de colateral con valoración vía oráculo, obligación de un solo token de colateral y liquidación proporcional con tope de cobertura
- Compuerta de activación del cierre de emergencia y una liquidación total final (`withdrawn_all`)
- Pausa y transferencia de administrador en dos pasos

## Cómo se testea

- `success_tests.rs` — 21 tests dirigidos de flujo extremo a extremo con aserciones exactas campo a campo de la contabilidad
- `error_tests.rs` — 44 tests, uno por guard de error de negocio
- `fuzz_properties.rs` — fuzzing basado en propiedades (proptest) de la matemática de asignación a través del entrypoint público `invest`, testeando invariantes de asignación y la ecuación contable sobre miles de casos aleatorios

Ver [Tests y cobertura](./tests.md) para los detalles.

## Qué falta aún para que Equillar esté listo para producción

Como mínimo:

1. Auditoría de seguridad externa
2. Corrección de los hallazgos de auditoría
3. Runbooks operativos más sólidos
4. Guías de despliegue y monitorización más claras
5. Validación de preparación para producción más allá de tests unitarios/integradores y basados en propiedades

## Nota importante

Que el protocolo esté **sin auditar** no es algo que deba esconderse. Forma parte del estado actual y debe comunicarse explícitamente a reviewers, contribuidores e integradores.
