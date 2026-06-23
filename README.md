# IMtBProcurement Ec (Immutable Procurement MVP)

Este repositorio contiene la implementación del **Producto Mínimo Viable (MVP)** para un **Sistema Inmutable y Transparente para la Fase Precontractual de Contratación Pública en Ecuador**, basado en una arquitectura descentralizada que combina la blockchain de **Solana**, un backend de orquestación en **Rust/Rocket** y almacenamiento descentralizado en **IPFS**.

---

## 🌟 Características Clave

*   **Persistencia Inmutable:** Registro seguro de convocatorias, ofertas y resoluciones en la red de Solana mediante Smart Contracts construidos con **Anchor**.
*   **Candados de Tiempo Criptográficos:** Garantiza por consenso de la red que ninguna postulación (oferta) sea admitida si se recibe un solo segundo después de la fecha límite establecida (*deadline*).
*   **Control del Techo Presupuestario (PAC):** Validación a nivel de contrato inteligente que impide la publicación de convocatorias que excedan el presupuesto del Plan Anual de Contratación (PAC) de la institución.
*   **Almacenamiento Descentralizado:** Subida automatizada de documentos en formato PDF/ZIP (Términos de Referencia y propuestas técnicas) a **IPFS** mediante el servicio de API JWT de **Pinata**.
*   **Auditoría y Transparencia:** Trazabilidad completa del ciclo de vida del proceso (`Convocatoria Abierta` → `Evaluación Técnica` → `Adjudicado`).
*   **Fondeo Automático (Airdrops):** El backend detecta si se está ejecutando localmente y financia las billeteras automáticamente con 2 SOL cada vez que se arranca.

---

## 🏗️ Arquitectura de la Solución

El proyecto está estructurado de forma modular en los siguientes componentes:

```
99_PFM_Traz_doc/
├── blockchain/          # Proyecto de Solana / Anchor (Smart Contract y Tests)
│   ├── programs/        # Programas de Solana escritos en Rust
│   │   └── blockchain/
│   │       ├── src/lib.rs # Código consolidado (lógica, cuentas y errores)
│   │       └── tests/   # Pruebas de integración nativas en Rust con LiteSVM
│   └── Anchor.toml      # Configuración de Anchor
├── backend/             # Servidor de API REST escrito en Rust con Rocket
│   ├── src/             # Controladores, clientes de Solana e IPFS
│   └── Cargo.toml       # Dependencias de Rust (Rocket, Solana SDK, base64, etc.)
├── frontend/            # Interfaz de usuario React/Vite (Dashboard de control)
│   ├── src/             # Componentes, vistas y estética Glassmorphism
│   └── package.json     # Configuración y dependencias de React/Vite (Lucide Icons, etc.)
└── README.md            # Guía de instalación y uso (Este archivo)
```

---

## 📊 Estructura de Datos en Solana (Capa Blockchain)

El programa inteligente (`blockchain/programs/blockchain/src/lib.rs`) define 4 cuentas principales indexadas mediante Program Derived Addresses (PDAs):

### 1. `Institution`
*   **Propósito:** Almacena el perfil y límites de presupuesto de la entidad contratante.
*   **Semilla PDA:** `[b"institution", authority.key().as_ref()]`
*   **Estructura:**
    *   `authority: Pubkey` - Wallet con permisos de administración.
    *   `name: String` - Nombre de la institución.
    *   `pac_budget_limit: u64` - Techo presupuestario anual (PAC).
    *   `pac_budget_spent: u64` - Presupuesto total comprometido a la fecha.

### 2. `ProcurementProcess`
*   **Propósito:** Representa la convocatoria pública inmutable.
*   **Semilla PDA:** `[b"process", id.to_le_bytes().as_ref()]`
*   **Estructura:**
    *   `id: u64` - Identificador numérico único del proceso.
    *   `institution: Pubkey` - Dirección de la institución convocante.
    *   `title: String` - Título del proceso de licitación.
    *   `referential_budget: u64` - Presupuesto referencial de la convocatoria.
    *   `deadline: i64` - Unix timestamp límite para la recepción de ofertas.
    *   `status: u8` - Estado (`0` = Convocatoria, `1` = Evaluación, `2` = Adjudicado).
    *   `ipfs_tdr_hash: String` - Enlace CID al documento TDR en IPFS.

### 3. `OfferAccount`
*   **Propósito:** Registra la propuesta de un proveedor.
*   **Semilla PDA:** `[b"offer", process.key().as_ref(), provider.key().as_ref()]`
*   **Estructura:**
    *   `provider: Pubkey` - Wallet del oferente.
    *   `process_id: u64` - ID del proceso asociado.
    *   `ipfs_proposal_hash: String` - Enlace CID a la propuesta técnica en IPFS.
    *   `submission_timestamp: i64` - Timestamp oficial de registro en Solana.
    *   `expertise_verified: bool` - Calificación técnica de cumplimiento de pliegos.

### 4. `AwardResolution`
*   **Propósito:** Acta de adjudicación del ganador del proceso.
*   **Semilla PDA:** `[b"resolution", process.key().as_ref()]`
*   **Estructura:**
    *   `process_id: u64` - ID del proceso.
    *   `winner_provider: Pubkey` - Wallet del proveedor adjudicado.
    *   `winning_bid_hash: String` - CID del documento final ganador en IPFS.
    *   `final_score_hash: String` - CID del informe final de evaluación en IPFS.
    *   `timestamp: i64` - Timestamp de la adjudicación.

---

## ⚙️ Reglas de Negocio y Validación On-Chain

El contrato inteligente aplica de manera determinista las siguientes restricciones:

1.  **Límite Presupuestario (PAC):** Al crear un proceso, la suma del presupuesto referencial y el presupuesto ya gastado de la institución no puede exceder el límite establecido (`pac_budget_limit`). En caso de superarlo, la transacción se revierte con `BudgetExceeded`.
2.  **Candado de Tiempo (Deadline):** No se permiten ofertas (`submit_offer`) si la hora actual registrada por el cluster de Solana (`Clock::get()?.unix_timestamp`) es superior al `deadline` del proceso. En caso contrario, se revierte con `OfferSubmissionClosed`.
3.  **Evaluación Justa:** Solo la institución creadora del proceso puede evaluar ofertas y adjudicar, y no puede calificar u otorgar la resolución antes de que el plazo de recepción (`deadline`) haya expirado oficialmente.

---

## 🧪 Pruebas de Integración (LiteSVM)

Las pruebas están escritas de forma nativa en Rust en la ruta `blockchain/programs/blockchain/tests/test_imtbprocurement.rs`. El set de pruebas valida de forma instantánea sin requerir un nodo validador completo corriendo en segundo plano:

*   **Caso Feliz:** Registro de una institución, publicación de un proceso de compra por un monto válido, y comprobación del incremento en el gasto del PAC de la institución.
*   **Caso Límite PAC:** Intento de crear un segundo proceso que excede el límite del PAC de la institución. El contrato aborta con la firma del error `BudgetExceeded`.

Para ejecutar las pruebas:
```bash
cd blockchain
cargo test
```

---

## 🛠️ Requisitos Previos

Antes de clonar e instalar el proyecto, asegúrate de contar con las siguientes herramientas instaladas:

1.  **Rust & Cargo:** (v1.75+) `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2.  **Solana CLI:** (v1.18+) `sh -c "$(curl -sSfL https://release.solana.com/stable/install)"`
3.  **Anchor CLI:** (v0.30+) `cargo install --git https://github.com/coral-xyz/anchor avm --locked --force` y luego `avm install latest`
4.  **Node.js & NPM:** (v18.0+) [Descarga oficial](https://nodejs.org/)

---

## 🚀 Guía de Instalación y Ejecución del Flujo Completo

Sigue estos pasos detallados para montar y probar todo el entorno local.

### Paso 1: Configurar Solana en Modo Localnet y Lanzar el Validador

1.  Establece tu configuración de Solana para usar el validador local:
    ```bash
    solana config set --url localhost
    ```
2.  Genera una wallet por defecto (si no la tienes):
    ```bash
    solana-keygen new --no-bip39-passphrase
    ```
3.  Lanza el validador local de Solana en una terminal separada:
    ```bash
    solana-test-validator
    ```

### Paso 2: Desplegar el Contrato Inteligente (Anchor)

1.  Dirígete a la carpeta del contrato:
    ```bash
    cd blockchain
    ```
2.  Compila el programa:
    ```bash
    anchor build
    ```
3.  Despliega el programa en tu red local:
    ```bash
    anchor deploy
    ```
    *Nota: Guarda el Program ID resultante si cambia respecto al por defecto.*

### Paso 3: Configurar y Lanzar el Backend (Rocket)

1.  Dirígete a la carpeta del backend:
    ```bash
    cd ../backend
    ```
2.  Crea tu archivo `.env` configurando tu token de Pinata (deja en blanco o coméntalo si deseas usar el simulador IPFS local integrado) e identificadores de red:
    ```env
    PINATA_JWT=tu_jwt_token_de_pinata_aqui
    SOLANA_RPC_URL=http://localhost:8899
    PROGRAM_ID=HR3UbH45KuTanX5yiNDnPnYXr19r7mNuzUPPtj6acDJJ
    ```
3.  Inicia el servidor Rocket:
    ```bash
    cargo run
    ```
    *El backend estará escuchando en `http://localhost:8000`. Al arrancar, financiará automáticamente con SOL las billeteras locales de los proveedores y de la autoridad en la red local.*

### Paso 4: Levantar la Interfaz de Usuario (Frontend React)

1.  Dirígete a la carpeta del frontend:
    ```bash
    cd ../frontend
    ```
2.  Instala las dependencias y arranca el servidor de desarrollo Vite:
    ```bash
    npm install
    ```
3.  Arranca el servidor web local:
    ```bash
    npm run dev
    ```
    *El panel estará disponible en `http://localhost:5173`.*

---

## 🛡️ Instrucciones para Pruebas Manuales (E2E) en el Navegador

Una vez que tengas el frontend en `http://localhost:5173`, el backend en `http://localhost:8000` y el validador en `http://localhost:8899`:

1.  **Inicialización:** En la pestaña **Dashboard**, introduce el nombre de la institución (ej. *Ministerio de Telecomunicaciones*) y haz clic en **Registrar Entidad**. Esto enviará la transacción a Solana y creará el registro PDA.
2.  **Crear Convocatoria:** Ve a la pestaña **Crear Proceso**, asigna un ID numérico (ej. `1`), introduce un presupuesto (ej. `10000` SOL), selecciona un tiempo límite corto (ej. *1 minuto*) y sube un archivo TDR. Haz clic en **Publicar Convocatoria en Solana**.
3.  **Enviar Propuestas:** Ve a la pestaña **Procesos y Ofertas**, haz clic en el proceso recién creado en la columna izquierda. En la parte inferior, verás el formulario de envío. Selecciona enviar como **Proveedor A**, selecciona un archivo de propuesta y haz clic en **Subir y Firmar Oferta**. Repite el proceso para el **Proveedor B**.
4.  **Auditoría y Adjudicación:** Espera a que expire el plazo límite (1 minuto). El estado del proceso cambiará a **Evaluación**. En la lista de ofertas recibidas, haz clic en **Aprobar Requisitos** para las propuestas válidas. Finalmente, en el formulario de adjudicación, selecciona al ganador (ej. *Proveedor A*), introduce un hash simulado para el presupuesto final y la puntuación, y haz clic en **Confirmar Adjudicación**.
