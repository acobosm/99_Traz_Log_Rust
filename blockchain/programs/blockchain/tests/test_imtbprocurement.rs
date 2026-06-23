use {
    anchor_lang::{
        solana_program::{instruction::Instruction, clock::Clock},
        prelude::Pubkey,
        InstructionData, ToAccountMetas,
    },
    litesvm::LiteSVM,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_keypair::Keypair,
    solana_transaction::versioned::VersionedTransaction,
};

#[test]
fn test_procurement_lifecycle() {
    let program_id = blockchain::id();
    let authority = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/blockchain.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap(); // 10 SOL

    // 1. Derivar PDA de la institucion
    let (institution_pda, _bump) = Pubkey::find_program_address(
        &[b"institution", authority.pubkey().as_ref()],
        &program_id,
    );

    // 2. Inicializar la institución
    let name = "Municipio de Quito".to_string();
    let pac_budget_limit: u64 = 100_000;

    let inst_instruction = Instruction::new_with_bytes(
        program_id,
        &blockchain::instruction::InitializeInstitution {
            name: name.clone(),
            pac_budget_limit,
        }.data(),
        blockchain::accounts::InitializeInstitution {
            institution: institution_pda,
            authority: authority.pubkey(),
            system_program: anchor_lang::solana_program::system_program::id(),
        }.to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[inst_instruction], Some(&authority.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&authority]).unwrap();

    let res = svm.send_transaction(tx);
    assert!(res.is_ok());

    // 3. Crear un proceso
    let process_id: u64 = 2026001;
    let (process_pda, _bump) = Pubkey::find_program_address(
        &[b"process", process_id.to_le_bytes().as_ref()],
        &program_id,
    );

    let process_title = "Adquisicion de Servidores".to_string();
    let referential_budget: u64 = 30_000;
    
    // Obtener tiempo del reloj (por defecto 0 en LiteSVM o tiempo Unix actual)
    let deadline = 10_000_000; // Un timestamp en el futuro lejano

    // Para evitar errores de deadline, configuramos el reloj en LiteSVM
    let mut clock = svm.get_sysvar::<Clock>();
    clock.unix_timestamp = 1000;
    svm.set_sysvar::<Clock>(&clock);

    let proc_instruction = Instruction::new_with_bytes(
        program_id,
        &blockchain::instruction::InitializeProcess {
            id: process_id,
            title: process_title.clone(),
            referential_budget,
            deadline,
            ipfs_tdr_hash: "QmXoypizjW3WknFixtdKL9bL721w1234567890abcdefGH".to_string(),
        }.data(),
        blockchain::accounts::InitializeProcess {
            process: process_pda,
            institution: institution_pda,
            authority: authority.pubkey(),
            system_program: anchor_lang::solana_program::system_program::id(),
        }.to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[proc_instruction], Some(&authority.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&authority]).unwrap();

    let res = svm.send_transaction(tx);
    assert!(res.is_ok());

    // 4. Intentar crear otro proceso que exceda el límite del PAC (límite: 100k, ya gastado: 30k, intentamos 80k -> total 110k)
    let process_id_fail: u64 = 2026002;
    let (process_pda_fail, _bump) = Pubkey::find_program_address(
        &[b"process", process_id_fail.to_le_bytes().as_ref()],
        &program_id,
    );

    let proc_instruction_fail = Instruction::new_with_bytes(
        program_id,
        &blockchain::instruction::InitializeProcess {
            id: process_id_fail,
            title: "Compra de Vehiculos".to_string(),
            referential_budget: 80_000, // 80k + 30k = 110k > 100k
            deadline,
            ipfs_tdr_hash: "QmXoypizjW3WknFixtdKL9bL721w1234567890abcdefGH".to_string(),
        }.data(),
        blockchain::accounts::InitializeProcess {
            process: process_pda_fail,
            institution: institution_pda,
            authority: authority.pubkey(),
            system_program: anchor_lang::solana_program::system_program::id(),
        }.to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[proc_instruction_fail], Some(&authority.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&authority]).unwrap();

    let res = svm.send_transaction(tx);
    assert!(res.is_err()); // Debe fallar por exceso de presupuesto
}
