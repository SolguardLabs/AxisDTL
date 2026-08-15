# Integración

## Interfaces disponibles

AxisDTL ofrece dos superficies:

1. API Rust mediante el crate `axis_dtl`.
2. CLI JSON consumible desde JavaScript mediante `AxisClient`.

El SDK no administra claves ni firma operaciones. Su función es validar tipos,
proyectar netting, evaluar capacidad y traducir la salida del runtime.

## Contrato del CLI

```bash
axis_dtl [routed|direct|batch|auction|snapshot]
```

- stdout contiene un único documento JSON válido;
- stderr contiene diagnóstico;
- `0` indica ejecución correcta;
- cualquier otro código indica rechazo;
- sin argumento equivale a `routed`.

El consumidor debe imponer timeout, límite de bytes y esquema. Nunca debe
interpretar salida parcial como confirmación.

## Campos del reporte

| Campo             | Tipo     | Significado                       |
| ----------------- | -------- | --------------------------------- |
| `scenario`        | string   | escenario ejecutado               |
| `network_id`      | integer  | dominio de red                    |
| `source_asset`    | object   | activo fuente y precisión         |
| `target_asset`    | object   | activo destino y precisión        |
| `order_ids`       | string[] | órdenes aceptadas                 |
| `transaction_ids` | string[] | settlements confirmados           |
| `balances`        | object   | balances operativos del escenario |
| `supply`          | object   | suministro agregado               |
| `surface`         | object   | venues, rutas, vaults y margen    |
| `journal_entries` | integer  | longitud del journal              |
| `state_digest`    | string   | digest hexadecimal de 32 bytes    |
| `conservation_ok` | boolean  | invariante contable               |

Los consumidores deben ignorar campos nuevos que no comprendan y rechazar la
ausencia o cambio de tipo de los campos requeridos.

## Adaptador de proceso

```js
import { spawn } from "node:child_process";
import { AxisClient } from "./sdk/AxisClient.js";

function executeAxis(args) {
    return new Promise((resolve, reject) => {
        const child = spawn("axis_dtl", args, {
            stdio: ["ignore", "pipe", "pipe"],
            windowsHide: true,
        });
        let stdout = "";
        let stderr = "";

        child.stdout.setEncoding("utf8");
        child.stderr.setEncoding("utf8");
        child.stdout.on("data", (chunk) => (stdout += chunk));
        child.stderr.on("data", (chunk) => (stderr += chunk));
        child.once("error", reject);
        child.once("close", (code) => {
            if (code === 0) resolve(stdout);
            else reject(new Error(`axis exited with code ${code}: ${stderr}`));
        });
    });
}

const client = new AxisClient(executeAxis);
const health = await client.health();
if (!health.conserved) throw new Error("accounting invariant rejected");
```

En un servicio permanente, el adaptador debe añadir timeout, cancelación,
límite de memoria y un máximo de salida.

## Importes

`asBigInt` acepta:

- `bigint` no negativo;
- `number` entero, seguro y no negativo;
- cadena decimal canónica sin signo.

Rechaza:

```text
-1
1.5
"+10"
"01"
"1e6"
Number.MAX_SAFE_INTEGER + 1
```

Los importes deben cruzar fronteras JSON como strings decimales para evitar
pérdida de precisión.

## Vista previa de netting

```js
import { previewNetting } from "./sdk/AxisClient.js";

const result = previewNetting(
    [
        { debtor: "a", creditor: "b", asset: "AXUSD", amount: "100", reference: "w80-1" },
        { debtor: "b", creditor: "c", asset: "AXUSD", amount: "70", reference: "w80-2" },
        { debtor: "c", creditor: "a", asset: "AXUSD", amount: "40", reference: "w80-3" },
    ],
    { AXUSD: "66" },
    1_000,
);

const [asset] = result.assets;
console.log(asset.gross); // 210n
console.log(asset.payable); // 60n
console.log(asset.compressionBps); // 7142
console.log(result.fullyReserved); // true
```

Antes de enviar la vista a otro sistema, serializar bigint de forma explícita:

```js
const json = JSON.stringify(result, (_, value) =>
    typeof value === "bigint" ? value.toString() : value,
);
```

## Capacidad de ruta

```js
import { planRouteCapacity } from "./sdk/AxisClient.js";

const capacity = planRouteCapacity(
    "7000000", // used
    "10000000", // limit
    "1500000", // requested
    "1000000", // protected reserve floor
);

if (!capacity.accepted) {
    throw new Error(capacity.reason);
}
```

La proyección es orientativa. Rust vuelve a comprobar el estado actual antes de
confirmar.

## API Rust

La librería exporta tipos de dominio desde `axis_dtl`. Ejemplo de netting:

```rust
use std::collections::BTreeMap;
use axis_dtl::{Amount, Bps, ClearingEngine, ClearingLimits};

let limits = ClearingLimits::new(128, 32, Bps::new(500)?)?;
let engine = ClearingEngine::new(limits);
let cycle = engine.preview(window, &obligations, &BTreeMap::new())?;

if !cycle.fully_reserved() {
    // Supply reserves before settlement.
}
```

La aplicación integradora debe tratar `AxisResult` como un rechazo esperado y
no convertirlo en `panic!`.

## Gobierno desde Rust

```rust
use axis_dtl::{ControlCommittee, ControlVoteKind};

let mut committee = ControlCommittee::new(2)?;
committee.register_reviewer(first_identity)?;
committee.register_reviewer(second_identity)?;

let digest = committee.submit(&signed_action)?;
committee.vote(&signed_approval)?;
let action = committee.execute(digest, current_epoch)?;
apply_control_action(action)?;
```

El ejemplo omite la construcción de claves. Las claves no deben guardarse en el
repositorio ni imprimirse en logs.

## Idempotencia

El consumidor correlaciona por `order_id`, `transaction_id` y `state_digest`.
Ante un timeout:

1. no incrementar el nonce de forma especulativa;
2. consultar o reconstruir el estado confirmado;
3. comparar transaction ID y digest;
4. reenviar solo si el nonce sigue disponible;
5. tratar un rechazo de repetición como señal de reconciliación.

## Compatibilidad

- Los campos existentes del reporte no cambian de tipo dentro de `1.x`.
- Se pueden añadir campos opcionales en versiones menores.
- Un cambio de semántica firmada requiere nuevo dominio.
- Un cambio incompatible del reporte requiere versión mayor.
- Cargo, npm y etiqueta deben declarar la misma versión.

## Checklist de integración

- [ ] Importes transportados como string o bigint.
- [ ] Timeout y límite de salida configurados.
- [ ] `state_digest` validado como 32 bytes hexadecimales.
- [ ] `conservation_ok` exigido antes de aceptar el reporte.
- [ ] stderr separado de stdout.
- [ ] IDs usados para idempotencia y correlación.
- [ ] Claves fuera del proceso de parsing y de los logs.
- [ ] Versión y commit registrados en telemetría.
- [ ] Respuesta de rechazo tratada sin reintento infinito.
