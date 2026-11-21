---
sidebar_position: 12
title: Pausa y reanudación
---

# Pausa y reanudación

## Introducción

Equillar usa la extensión `Pausable` de OpenZeppelin para proporcionar un switch de seguridad de propósito general. Cuando el contrato está pausado, prácticamente todas las funciones con cambio de estado quedan bloqueadas, protegiendo el contrato mientras se investiga un incidente (un bug sospechado, un comportamiento inesperado, etc.).

Tanto `pause` como `unpause` solo pueden ser ejecutadas por el administrador.

### Comportamiento del guard por función

| Función | Acceso (sin pausa) | Comportamiento con pausa |
|---|---|---|
| `invest` | `operator` (dirección inversora) | Bloqueada |
| `enable_investment_liquidations` | admin | Bloqueada |
| `disable_investment_liquidations` | admin | Bloqueada |
| `check_investment_liquidations` | admin o cualquier rol | **Permitida** (read-only) |
| `process_investor_payment` | admin | Bloqueada |
| `refund_investor` | admin | Bloqueada |
| `get_contract_balance` | admin o cualquier rol | **Permitida** (read-only) |
| `grant_operator` / `revoke_operator` | admin | Bloqueadas |
| `grant_company` / `revoke_company` | admin | Bloqueadas |
| `grant_manager` / `revoke_manager` | admin | Bloqueadas |
| `transfer_admin_role` / `accept_admin_transfer_role` | admin | Bloqueadas |
| `withdrawn` | admin, `-to` con rol `company` | Bloqueada |
| `withdrawn_commissions` | admin, `-to` con rol `manager` | Bloqueada |
| `withdrawn_all` | admin, `-to` con rol `manager` | Bloqueada |
| `withdrawn_all_collateral` | admin, `-to` con rol `manager` | Bloqueada |
| `add_company_transfer` | `-from` con rol `company` | Bloqueada |
| `activate_emergency_close` | admin | Bloqueada |
| `add_collateral` | `collateral_addr` con rol `company` | Bloqueada |
| `pay_with_collateral` | admin | Bloqueada |
| `return_collateral_to_company` | admin | Bloqueada |
| `paused` (lectura) | cualquiera | **Permitida** (read-only) |
| `upgrade` | admin | **Solo mientras está pausado** |

Como regla general, toda función que mueve fondos, cambia roles o altera la configuración del contrato está bloqueada durante la pausa. Las únicas excepciones son las funciones read-only (`get_contract_balance`, `check_investment_liquidations`, `paused`), que permanecen disponibles para que el estado del contrato pueda inspeccionarse durante un incidente, y `upgrade`, que **solo** puede llamarse con el contrato pausado — garantizando que el código del contrato no pueda cambiar a mitad de transacción o mientras otras operaciones pueden seguir en vuelo.

## Por qué nada que mueve fondos queda exento

A diferencia del [cierre de emergencia](./emergency-close.md), una compuerta terminal acotada a las obligaciones pendientes de settlement, la `pausa` pretende ser un stop total que cubre *toda* operación que mueve fondos, incluidas las orientadas a inversores como `refund_investor`. Es intencional: si el motivo de la pausa es un problema sospechado con la lógica o contabilidad del contrato, permitir la ejecución de cualquier función que mueva fondos socavaría la garantía de seguridad que la pausa pretende proporcionar. Una vez confirmado que el contrato es seguro, el admin puede reanudar la operación normal, y cualquier reembolso, pago u otra operación pendiente puede continuar con normalidad.

## Interacción pausa ↔ emergencia

La pausa y el cierre de emergencia son mecanismos separados con ámbitos separados:

- **La pausa** es administrativa y reversible (`unpause`), cubre todo incluyendo la propia compuerta de emergencia, y es la herramienta para incidentes sospechados a nivel de contrato.
- **El cierre de emergencia** es terminal de negocio: una vez activado no puede desactivarse, y congela los flujos de capital con independencia del estado de pausa.

Con el contrato pausado, `activate_emergency_close` está bloqueada; con la emergencia activa, todos los entrypoints que mueven capital (incluidos los protegidos por la pausa) rechazan con `OperationNotAllowedInEmergency` (50).
