# IMtBProcurement Ec (Immutable Procurement DApp)

Este repositorio contiene la implementación del **Producto Mínimo Viable (MVP)** para un **Sistema Inmutable y Transparente para la Fase Precontractual de Contratación Pública en Ecuador**, basado en una arquitectura descentralizada Web3 real que combina la blockchain de **Solana**, un cliente **Anchor (JavaScript)** integrado en el frontend, un backend microservicio en **Rust/Rocket** para IPFS y almacenamiento descentralizado.

---

## 🌟 Características Clave

*   **Persistencia Inmutable:** Registro seguro de convocatorias, ofertas y resoluciones en la red de Solana mediante Smart Contracts construidos con **Anchor**.
*   **Firma Digital Descentralizada (Web3):** Integración con **Solana Wallet Adapter** en el navegador para que los usuarios (Autoridad e Instituciones/Proveedores) firmen transacciones directamente con billeteras Web3 reales como **Phantom** o **Solflare**.
*   **Candados de Tiempo Criptográficos:** Garantiza por consenso de la red que ninguna postulación (oferta) sea admitida si se recibe después de la fecha límite establecida (*deadline*).
*   **Control del Techo Presupuestario (PAC):** Validación a nivel de contrato inteligente que impide la publicación de convocatorias que excedan el presupuesto del Plan Anual de Contratación (PAC) de la institución.
*   **Almacenamiento Descentralizado Seguro:** Subida de documentos en formato PDF/ZIP (Términos de Referencia y propuestas técnicas) a **IPFS** cifrados temporalmente o guardados a través del backend para proteger las credenciales JWT de **Pinata**.
*   **Auditoría y Transparencia:** Trazabilidad completa del ciclo de vida del proceso (`Convocatoria Abierta` → `Evaluación Técnica` → `Adjudicado`).

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
├── backend/             # Servidor de API / Puente IPFS escrito en Rust con Rocket
│   ├── src/             # Controladores y clientes de IPFS/Pinata
│   └── Cargo.toml       # Dependencias de Rust (Rocket, reqwest, base64, uuid, etc.)
├── frontend/            # Interfaz de usuario React/Vite (DApp Web3 Real)
│   ├── src/             # Componentes, vistas y estética Glassmorphism
│   │   ├── idl.json     # Copia del IDL autogenerado por Anchor
│   │   ├── main.jsx     # Configuración de los adaptadores de Wallet
│   │   └── App.jsx      # Lógica de la DApp y llamadas al programa Solana
│   └── package.json     # Configuración y dependencias de React/Vite (Wallet adapters, Anchor Client, etc.)
└── README.md            # Guía de instalación y uso (Este archivo)
```

---

## 📊 Estructura de Datos en Solana (Capa Blockchain)

El programa inteligente (`blockchain/programs/blockchain/src/lib.rs`) define 4 cuentas principales indexadas mediante Program Derived Addresses (PDAs):

### 1. `Institution`
*   **Propósito:** Almacena el perfil y límites de presupuesto de la entidad contratante.
*   **Semilla PDA:** `[b"institution", authority.key().as_ref()]`
*   **Estructura:**
    *   `authority: Pubkey` - Wallet del administrador de la institución.
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

```bash
cd blockchain
cargo test
```

---

## ⚙️ Configuración Requerida en Phantom Wallet

Para interactuar con la DApp en tu entorno localnet, debes configurar tu extensión de Phantom:

1.  **Habilitar el Modo de Desarrollador:**
    *   Abre la extensión **Phantom**.
    *   Ve a **Configuración** (icono de engranaje ⚙️ en la esquina inferior derecha).
    *   Selecciona **Configuración del desarrollador** (Developer Settings).
    *   Activa la casilla **Modo de desarrollador** (Developer Mode).
2.  **Cambiar de Red a Localhost:**
    *   Dentro de la misma pantalla de **Configuración del desarrollador**, selecciona **Cambiar red** (Change Network).
    *   Selecciona **Localhost** (con la dirección `http://127.0.0.1:8899` o `http://localhost:8899`).
3.  **Fondeo de SOL locales a tu Wallet:**
    *   Copia la dirección de tu clave pública de Phantom (haz clic en el nombre de tu cuenta arriba en la extensión para copiarla).
    *   Abre tu consola de comandos del sistema (con el validador local de Solana encendido) y ejecuta:
        ```bash
        solana airdrop 5 TU_DIRECCION_DE_PHANTOM
        ```
    *   Verás reflejados 5 SOL en tu Phantom en la red local. *(Nota: También puedes usar el botón de Airdrop directamente en el Dashboard de la DApp si deseas).*
4.  **Crear Múltiples Cuentas para Probar Roles:**
    *   En Phantom, haz clic en el menú superior izquierdo (donde dice el nombre de tu cuenta).
    *   Haz clic en **"Añadir / Conectar cartera"** (Add/Connect Wallet).
    *   Selecciona **"Crear nueva cuenta"** (Create a new account).
    *   Crea al menos dos cuentas adicionales (ej: *Wallet 2* y *Wallet 3*).
    *   Usa una cuenta como **Autoridad Pública** (para registrar la institución y crear licitaciones) y las otras cuentas como **Proveedores** (para enviar ofertas).
    *   *Recuerda fondear con Airdrop las cuentas que vayas a usar.*

---

## 🚀 Guía de Instalación y Ejecución del Flujo Completo

### Paso 1: Configurar Solana en Modo Localnet y Lanzar el Validador

1.  Establece tu configuración de Solana para usar el validador local:
    ```bash
    solana config set --url localhost
    ```
2.  Lanza el validador local de Solana en una terminal separada:
    ```bash
    solana-test-validator
    ```

### Paso 2: Desplegar el Contrato Inteligente (Anchor)

1.  Dirígete a la carpeta del contrato:
    ```bash
    cd blockchain
    ```
2.  Compila y despliega el programa:
    ```bash
    anchor build && anchor deploy
    ```
3.  Copia el IDL compilado al frontend:
    ```bash
    cp target/idl/blockchain.json ../frontend/src/idl.json
    ```

### Paso 3: Configurar y Lanzar el Backend (Puente IPFS)

1.  Dirígete a la carpeta del backend:
    ```bash
    cd ../backend
    ```
2.  Crea tu archivo `.env` configurando tu token de Pinata (deja en blanco o coméntalo si deseas usar el simulador IPFS local integrado) e identificadores de red:
    ```env
    PINATA_JWT=tu_jwt_token_de_pinata_aqui
    ```
3.  Inicia el servidor Rocket:
    ```bash
    cargo run
    ```
    *El backend estará escuchando en `http://localhost:8000` sirviendo como puente CORS de subida a IPFS.*

### Paso 4: Levantar la Interfaz de Usuario (Frontend React)

1.  Dirígete a la carpeta del frontend:
    ```bash
    cd ../frontend
    ```
2.  Instala las dependencias y arranca el servidor de desarrollo Vite:
    ```bash
    npm install && npm run dev
    ```
    *La DApp estará disponible en `http://localhost:5173`.*

---

## 🛡️ Instrucciones para Pruebas Manuales (E2E) en el Navegador

1.  **Conexión inicial:** Abre `http://localhost:5173` y conecta tu Phantom Wallet usando el botón **"Select Wallet"** en la cabecera.
2.  **Registrar Institución (Autoridad):** Asegúrate de tener SOL local en tu cuenta de Phantom. En la pestaña **Dashboard**, introduce el nombre de la institución (ej. *Ministerio de Telecomunicaciones*) y haz clic en **Registrar Entidad Gubernamental**. Confirma la firma en la ventana emergente de Phantom.
3.  **Convocar Licitación:** Ve a la pestaña **Crear Proceso** (se habrá activado tras registrar la institución). Asigna un ID numérico (ej. `1`), introduce un presupuesto (ej. `1200000` SOL), selecciona un tiempo límite corto (ej. *1 Minuto*) y sube un pliego TDR. Haz clic en **Publicar Convocatoria en Solana** y firma con tu Phantom.
4.  **Enviar Propuestas (Proveedores):**
    *   Abre Phantom y cambia a una cuenta diferente de proveedor (ej: *Wallet 2*).
    *   Haz clic en la pestaña **Procesos y Ofertas**, selecciona la licitación `#1` en la lista.
    *   En el formulario de presentación de propuestas, sube el archivo de tu propuesta y haz clic en **Subir y Firmar Oferta**. Confirma la firma en Phantom.
    *   *Opcional:* Cambia a otra cuenta en Phantom (ej: *Wallet 3*) y envía otra oferta para simular competencia.
5.  **Auditoría y Adjudicación:**
    *   Espera a que expire el plazo límite (1 minuto).
    *   Cambia de nuevo en Phantom a la cuenta de la **Autoridad** (Wallet 1).
    *   Actualiza el detalle del proceso. El estado habrá cambiado a **Evaluación Técnica**.
    *   Como Autoridad, evalúa las ofertas haciendo clic en **Aprobar Requisitos** (aprobando o rechazando y firmando cada acción con Phantom).
    *   En el formulario inferior de adjudicación, selecciona la dirección ganadora, introduce los hashes CID del acta y del presupuesto definitivo de IPFS, y haz clic en **Confirmar Adjudicación**. Firma en Phantom.
    *   ¡Listo! El proceso de compra habrá concluido y los fondos PAC quedarán devengados on-chain.
