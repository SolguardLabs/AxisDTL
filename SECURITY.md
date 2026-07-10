# Seguridad de AxisDTL

AxisDTL modela un protocolo de liquidacion con controles defensivos aplicados en
varias capas: identidad criptografica, serializacion canonica, nonces, rutas,
oraculo, limites de riesgo, custodia y conservacion contable.

Este documento describe el modelo de seguridad esperado para operar y revisar el
repositorio de forma profesional.

## Alcance

Estan dentro del alcance:

- logica Rust en `src/`;
- escenarios CLI expuestos por `runtime`;
- scripts de validacion en `scripts/`;
- tests Rust y Node en `tests/`;
- workflows de GitHub Actions;
- configuracion de dependencias en `Cargo.toml`, `Cargo.lock` y `package.json`.

Quedan fuera del alcance:

- despliegues en redes publicas;
- custodia de claves reales;
- integraciones con bridges, RPCs, indexadores o servicios externos;
- automatizaciones que no formen parte de este repositorio.

## Modelo de Confianza

El protocolo asume que:

- las identidades se derivan de claves Ed25519 validas;
- toda autorizacion relevante se firma sobre payloads canonicos;
- los nonces de cuenta avanzan de forma monotona;
- los saldos se actualizan de forma atomica;
- los activos registrados mantienen metadatos estables durante la ejecucion;
- el oraculo configurado publica observaciones dentro de una ventana temporal
  aceptada;
- las rutas registradas representan venues habilitados y tramos continuos;
- los perfiles de riesgo y vaults forman parte del estado auditado.

## Controles Implementados

### Firmas y Autorizacion

Las ordenes de swap y las solicitudes de liquidacion se firman con dominios
separados. Cada payload usa serializacion canonica antes de firmarse o
verificarse, evitando ambiguedades entre representaciones equivalentes.

### Identificadores Deterministas

Las cuentas, activos, ordenes, transacciones y snapshots se identifican mediante
digests de dominio. Esto permite reproducibilidad, comparacion entre ejecuciones
y trazabilidad en el journal.

### Nonces

El ledger mantiene nonces independientes para pagadores y solvers. Las
operaciones con nonce inesperado se rechazan antes de modificar estado.

### Ejecucion Atomica

La ejecucion de swaps se aplica sobre una copia candidata del ledger. El estado
principal solo se reemplaza si todas las validaciones y movimientos contables
finalizan correctamente.

### Conservacion Contable

Despues de cada movimiento relevante, el ledger compara el suministro esperado
por activo con la suma observada en todas las cuentas registradas.

### Rutas y Venues

El route book exige venues registrados, rutas no vacias, continuidad entre
tramos, liquidez suficiente y limites de hops. La calidad de ruta queda
registrada como parte del evento de ejecucion.

### Oraculo y Bandas de Precio

El registro de oraculo controla publicadores autorizados, frescura de
observaciones y desviacion maxima aceptada frente a la cotizacion ejecutada.

### Riesgo y Custodia

El motor de riesgo valida limites de importe, salida, comision, hops y perfil de
cuenta. La capa de custodia registra vaults, reservas, politica de tesoreria y
cuentas de margen.

## Verificacion Local

Antes de proponer cambios, ejecutar:

```bash
cargo fmt --all -- --check
cargo check --locked
cargo test --locked
node --test tests/node/*.test.js
```

Tambien puede usarse:

```bash
bash scripts/ci.sh
```

En Windows, si Bash no dispone de Node o Cargo en `PATH`, ejecutar los comandos
anteriores desde PowerShell.

## Requisitos de Cambios

Los cambios que afecten liquidacion, saldos, firmas, rutas, oraculo, riesgo o
custodia deben incluir:

- tests de escenario actualizados;
- validacion de conservacion;
- revision de serializacion canonica cuando cambien payloads firmados;
- confirmacion de que los reportes JSON siguen siendo estables para consumidores
  externos.

Los cambios en workflows o scripts deben poder ejecutarse localmente o tener una
justificacion clara de entorno.

## Reporte Responsable

Para reportar un hallazgo, incluir:

- descripcion tecnica;
- pasos de reproduccion;
- comandos ejecutados;
- escenario afectado;
- impacto observado;
- propuesta de mitigacion, si existe.

No incluir claves privadas, semillas reales, credenciales, endpoints privados ni
datos de terceros. Este repositorio usa fixtures deterministas y no necesita
secretos para reproducir su comportamiento.
