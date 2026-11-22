---
sidebar_position: 1
title: Introducción
---

# ¿Qué es Equillar?

Equillar es un **motor de ciclo de vida de inversiones** implementado como un contrato Soroban modular sobre Stellar.

Su propósito es ayudar a un integrador a operar flujos de financiación entre:

- **Integradores**: Fintechs, exchanges o plataformas que integran Equillar en sus servicios
- **Empresas**: Empresas que solicitan financiación para un proyecto concreto
- **Usuarios**: Usuarios que invierten a través de la plataforma del integrador

## Qué hace Equillar

A nivel de contrato, Equillar proporciona:

- Control de acceso basado en roles (`admin`, `operator`, `company`, `manager`)
- Aportaciones de capital mediante `invest`, con un modelo de comisión dinámica (dependiente del tamaño)
- Gestión de posiciones identificadas por un identificador (`position_id`) proporcionado por el integrador
- Contabilidad de partida doble (`Balance`) y verificada (activos = pasivos + patrimonio neto) tras cada operación
- Rondas de pago programadas, con transferencias dentro del contrato o un modo de pago fuera del contrato
- Gestión opcional de colateral con valoración vía oráculo y liquidación proporcional
- Cierre de emergencia y liquidación final completa (`withdrawn_all`)
- Control de pausa y transferencia del rol de administrador en dos pasos

## Qué no hace Equillar

Equillar **no** provee de:

- Estatus regulatorio
- Flujos de compliance
- KYC / AML ni onboarding
- Estructuración legal de ofertas
- UX de producto orientada al consumidor por sí mismo

Esas responsabilidades recaen en el **integrador** y en los sistemas off-chain que opere.

## Por qué el motor se basa en roles

Equillar **no** es una primitiva **totalmente permissionless**, sino que está diseñado para operar en entornos gestionados donde se espera que el integrador controle acciones críticas como:

- Asignación de roles (admin)
- Gestión de entradas de capital mediante operadores aprobados
- Transferencias de fondos de la empresa para rondas de pago
- Depósitos de colateral
- Activación de emergencia
- Retiros, pagos y liquidación final

Esto encaja con la visión actual del producto: **Equillar provee la capa tecnológica subyacente para integradores que ya están regulados.**

## Índice de contenidos

La documentación sigue el ciclo de vida del protocolo.

| # | Sección | Contenido |
|---|---|---|
| 1 | [Estado actual](./current-status.md) | Estado de implementación, qué está testado, qué falta para producción |
| 2 | [Despliegue del contrato](./deploying-the-contract.md) | Build, identidades, upload, deploy, parámetros del constructor, timeline del ciclo de vida |
| 3 | [El modelo de Balance](./balance-model.md) | Contabilidad de partida doble: la ecuación, cada campo, cómo los mueve cada operación |
| 4 | [Configuración y revocación de roles](./roles.md) | Los cuatro roles, flujos de grant/revoke, regla de titular único |
| 5 | [Fundraising y posiciones](./fundraising-and-positions.md) | `invest`, el modelo de tramos de comisión, ejemplos de asignación, estados de la posición |
| 6 | [Reembolsos (refunds)](./refunds.md) | `refund_investor` durante el periodo de `fundraising`, reversión completa de la comisión |
| 7 | [Rondas de pago](./payment-rounds.md) | Transferencias de la empresa, pagos por posición, modo sin transferencias, tipos de pago: coupon/reverse-loan |
| 8 | [Retiradas](./withdrawals.md) | Retiradas de fondos, retiro de comisiones, la operación `withdrawn_all` final |
| 9 | [Colateral](./collateral.md) | Depósitos, valoración vía oráculo, liquidación proporcional con tope de cobertura |
| 10 | [Cierre de emergencia](./emergency-close.md) | La compuerta terminal de activación y cómo se combina con `withdrawn_all` |
| 11 | [Pausa y reanudación](./admin-controls.md) | El switch de seguridad Pausable, guards por función |
| 12 | [Tests y cobertura](./tests.md) | Los tres binarios de test, fuzzing basado en propiedades, qué cubre y qué no |
| — | [Apéndice: códigos de error](./error-codes.md) | Los códigos de error de negocio y cuándo se dispara cada uno |

Dos secciones son especialmente importantes como trasfondo del resto: [Estado actual](./current-status.md) y [El modelo de Balance](./balance-model.md); la contabilidad de partida doble a la que hace referencia paso a paso cada sección posterior.
