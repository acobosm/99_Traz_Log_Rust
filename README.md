# IMtBProcurement Ec (Immutable Procurement MVP)

Este repositorio contiene la implementación del **Producto Mínimo Viable (MVP)** para un **Sistema Inmutable y Transparente para la Fase Precontractual de Contratación Pública en Ecuador**, basado en una arquitectura descentralizada que combina la blockchain de **Solana**, un backend de orquestación en **Rust/Rocket** y almacenamiento descentralizado en **IPFS**.

---

## 🌟 Características Clave

*   **Persistencia Inmutable:** Registro seguro de convocatorias, ofertas y resoluciones en la red de Solana mediante Smart Contracts construidos con **Anchor**.
*   **Candados de Tiempo Criptográficos:** Garantiza por consenso de la red que ninguna postulación (oferta) sea admitida si se recibe un solo segundo después de la fecha límite establecida (*deadline*).
*   **Control del Techo Presupuestario (PAC):** Validación a nivel de contrato inteligente que impide la publicación de convocatorias que excedan el presupuesto del Plan Anual de Contratación (PAC) de la institución.
*   **Almacenamiento Descentralizado:** Subida automatizada de documentos en formato PDF (Términos de Referencia y propuestas técnicas) a **IPFS** mediante el servicio de pinning de **Pinata**.
*   **Auditoría y Transparencia:** Trazabilidad completa del ciclo de vida del proceso (`Convocatoria Abierta` → `Recepción Cerrada` → `Proceso Adjudicado`).

---

## 🏗️ Arquitectura de la Solución

El proyecto está estructurado de forma modular en tres componentes principales:

```
99_PFM_Traz_doc/
├── blockchain/          # Proyecto de Solana / Anchor (Smart Contract y Tests)
│   ├── programs/        # Programas de Solana escritos en Rust
│   ├── tests/           # Pruebas de integración en TypeScript
│   └── Anchor.toml      # Configuración de Anchor
├── backend/             # Servidor de API REST escrito en Rust con Rocket
│   ├── src/             # Controladores, clientes de Solana e IPFS
│   └── Cargo.toml       # Dependencias de Rust (Rocket, Anchor client, etc.)
├── frontend/            # Interfaz de usuario web responsiva (Dashboard de control)
│   ├── index.html       # Estructura de la interfaz
│   ├── style.css        # Estilos premium (Glassmorphism & Neon accent)
│   └── app.js           # Orquestador del cliente web3 y llamadas al backend
└── README.md            # Guía de instalación y uso (Este archivo)
```

---

## 🛠️ Requisitos Previos

Antes de clonar e instalar el proyecto, asegúrate de contar con las siguientes herramientas instaladas:

1.  **Rust & Cargo:** (v1.75+) `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2.  **Solana CLI:** (v1.18+) `sh -c "$(curl -sSfL https://release.solana.com/stable/install)"`
3.  **Anchor CLI:** (v0.29+) `cargo install --git https://github.com/coral-xyz/anchor avm --locked --force` y luego `avm install latest`
4.  **Node.js & NPM:** (v18.0+) [Descarga oficial](https://nodejs.org/)

---

## 🚀 Guía de Instalación y Configuración

Sigue estos pasos detallados para montar el entorno de desarrollo local.

### Paso 1: Configurar Solana en Modo Localnet

1.  Establece tu configuración de Solana para usar el validador local:
    ```bash
    solana config set --url localhost
    ```
2.  Genera una wallet por defecto (si no la tienes):
    ```bash
    solana-keygen new --no-bip39-passphrase
    ```
3.  Inicia el validador local de Solana en una terminal dedicada:
    ```bash
    solana-test-validator
    ```

### Paso 2: Compilar y Desplegar el Contrato Inteligente (Anchor)

1.  Dirígete a la carpeta del contrato:
    ```bash
    cd blockchain
    ```
2.  Instala las dependencias de TypeScript para las pruebas:
    ```bash
    npm install
    ```
3.  Compila el programa:
    ```bash
    anchor build
    ```
4.  Obtén la dirección del programa compilado (`Program ID`):
    ```bash
    solana address -k target/deploy/imtbprocurement-keypair.json
    ```
5.  Actualiza esta clave en el archivo `Anchor.toml` y en `programs/imtbprocurement/src/lib.rs` en la macro `declare_id!`.
6.  Despliega el contrato en tu validador local:
    ```bash
    anchor deploy
    ```

### Paso 3: Configurar y Correr el Backend (Rocket)

1.  Dirígete a la carpeta del backend:
    ```bash
    cd ../backend
    ```
2.  Crea un archivo `.env` para la integración con IPFS/Pinata (si no se proporciona, el servidor entrará en modo simulación/mock local de archivos):
    ```env
    PINATA_JWT=tu_jwt_token_de_pinata
    SOLANA_RPC_URL=http://localhost:8899
    PROGRAM_ID=Direccion_De_Tu_Program_ID
    ```
3.  Inicia el servidor Rocket en modo desarrollo:
    ```bash
    cargo run
    ```
    *El backend estará escuchando en `http://localhost:8000`.*

### Paso 4: Levantar la Interfaz de Usuario (Frontend)

1.  Dirígete a la carpeta del frontend:
    ```bash
    cd ../frontend
    ```
2.  Puedes servir los archivos estáticos usando cualquier servidor local simple (por ejemplo, Live Server en VS Code o usando Python):
    ```bash
    python3 -m http.server 3000
    ```
3.  Abre el navegador en `http://localhost:3000` para interactuar con la aplicación.

---

## 🧪 Flujo de Pruebas y Simulación

Puedes validar el ciclo de vida completo siguiendo este flujo estructurado:

1.  **Sembrar Instituciones:** Llama al endpoint `/api/institutions` para registrar las cuentas de prueba ("Municipio de Quito", "Prefectura del Guayas") con sus límites presupuestarios del PAC.
2.  **Publicar Convocatoria:** Desde la interfaz, crea un proceso asignando un presupuesto referencial (ej. $15,000) y un límite de tiempo (deadline en segundos). Sube los TDRs en PDF.
3.  **Postulación de Proveedores:**
    *   **Oferta 1 (Dentro de plazo):** Sube la oferta técnica del Proveedor A. El sistema generará su hash CID y la registrará con éxito en Solana.
    *   **Oferta 2 (Fuera de plazo):** Cambia el reloj o espera a que expire el deadline e intenta subir la oferta del Proveedor B. La red Solana revertirá la transacción arrojando el error `OfferSubmissionClosed`.
4.  **Cierre y Adjudicación:** Una vez expirado el plazo, ejecuta la fase de evaluación. El sistema verificará los requisitos, descartará proveedores con `expertise_verified = false`, adjudicará al mejor postor calificado y guardará el acta inmutable en Solana en la cuenta `AwardResolution`.
