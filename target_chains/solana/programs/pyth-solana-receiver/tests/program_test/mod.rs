use {
    solana_program::{
        bpf_loader_upgradeable::{
            self,
            UpgradeableLoaderState,
        },
        hash::Hash,
        instruction::Instruction,
        native_token::LAMPORTS_PER_SOL,
        pubkey::Pubkey,
        rent::Rent,
        stake_history::Epoch,
        system_instruction,
    },
    solana_program_test::{
        read_file,
        BanksClient,
        BanksClientError,
        ProgramTest,
        ProgramTestBanksClientExt,
    },
    solana_sdk::{
        account::Account,
        signature::{
            Keypair,
            Signer,
        },
        transaction::Transaction,
    },
    std::path::PathBuf,
};

lazy_static::lazy_static! {
    // Build the oracle binary and make it available to the
    // simulator. lazy_static makes this happen only once per test
    // run.
    static ref PYTH_RECEIVER_PROGRAM_BINARY_PATH: PathBuf = {

    // Detect features and pass them onto cargo-build-bpf.
    // IMPORTANT: All features of this crate must have gates added to this vector.
    let features: Vec<&str> = vec![
    #[cfg(feature = "devnet")]
        "devnet",
    #[cfg(feature = "mainnet")]
        "mainnet",
    ];

    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("build-bpf");

    if !features.is_empty() {
        cmd.arg("--features");
        cmd.args(features);
    }

    let status = cmd.status().unwrap();

    if !status.success() {
        panic!(
        "cargo-build-bpf did not exit with 0 (code {:?})",
        status.code()
        );
    }

    let target_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target");

    PathBuf::from(target_dir).join("deploy/pyth_solana_receiver.so")
    };
}

/// Simulator for the state of the target chain program on Solana. You can run solana transactions against
/// this struct to test how pyth instructions execute in the Solana runtime.
pub struct ProgramSimulator {
    pub program_id:        Pubkey,
    banks_client:          BanksClient,
    /// Hash used to submit the last transaction. The hash must be advanced for each new
    /// transaction; otherwise, replayed transactions in different states can return stale
    /// results.
    last_blockhash:        Hash,
    pub upgrade_authority: Keypair,
    pub genesis_keypair:   Keypair,
}

impl ProgramSimulator {
    /// Deploys the target chain contract as upgradable
    pub async fn new() -> ProgramSimulator {
        let mut bpf_data = read_file(&*PYTH_RECEIVER_PROGRAM_BINARY_PATH);

        let mut program_test = ProgramTest::default();

        // This PDA is the actual address in the real world
        // https://docs.rs/solana-program/1.6.4/solana_program/bpf_loader_upgradeable/index.html
        let (programdata_address, _) = Pubkey::find_program_address(
            &[&pyth_solana_receiver::ID.to_bytes()],
            &bpf_loader_upgradeable::id(),
        );

        let upgrade_authority_keypair = Keypair::new();

        let program_deserialized = UpgradeableLoaderState::Program {
            programdata_address,
        };
        let programdata_deserialized = UpgradeableLoaderState::ProgramData {
            slot:                      1,
            upgrade_authority_address: Some(upgrade_authority_keypair.pubkey()),
        };

        // Program contains a pointer to programdata
        let program_vec = bincode::serialize(&program_deserialized).unwrap();
        // Programdata contains a header and the binary of the program
        let mut programdata_vec = bincode::serialize(&programdata_deserialized).unwrap();
        programdata_vec.append(&mut bpf_data);

        let program_account = Account {
            lamports:   Rent::default().minimum_balance(program_vec.len()),
            data:       program_vec,
            owner:      bpf_loader_upgradeable::ID,
            executable: true,
            rent_epoch: Epoch::default(),
        };
        let programdata_account = Account {
            lamports:   Rent::default().minimum_balance(programdata_vec.len()),
            data:       programdata_vec,
            owner:      bpf_loader_upgradeable::ID,
            executable: false,
            rent_epoch: Epoch::default(),
        };

        // Add to both accounts to program test, now the program is deploy as upgradable
        program_test.add_account(pyth_solana_receiver::ID, program_account);
        program_test.add_account(programdata_address, programdata_account);

        // Start validator
        let (banks_client, genesis_keypair, recent_blockhash) = program_test.start().await;

        let mut result = ProgramSimulator {
            program_id: pyth_solana_receiver::ID,
            banks_client,
            last_blockhash: recent_blockhash,
            upgrade_authority: upgrade_authority_keypair,
            genesis_keypair,
        };

        // Transfer money to upgrade_authority so it can call the instructions
        result
            .airdrop(&result.upgrade_authority.pubkey(), 1000 * LAMPORTS_PER_SOL)
            .await
            .unwrap();

        result
    }

    /// Process a transaction containing `instruction` signed by `signers`.
    /// `payer` is used to pay for and sign the transaction.
    pub async fn process_ix(
        &mut self,
        instruction: Instruction,
        signers: &Vec<&Keypair>,
        payer: &Keypair,
    ) -> Result<(), BanksClientError> {
        let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));

        let blockhash = self
            .banks_client
            .get_new_latest_blockhash(&self.last_blockhash)
            .await
            .unwrap();
        self.last_blockhash = blockhash;

        transaction.partial_sign(&[payer], self.last_blockhash);
        transaction.partial_sign(signers, self.last_blockhash);

        self.banks_client.process_transaction(transaction).await
    }

    /// Send `lamports` worth of SOL to the pubkey `to`.
    pub async fn airdrop(&mut self, to: &Pubkey, lamports: u64) -> Result<(), BanksClientError> {
        let instruction =
            system_instruction::transfer(&self.genesis_keypair.pubkey(), to, lamports);

        self.process_ix(instruction, &vec![], &self.genesis_keypair.insecure_clone())
            .await
    }
}
