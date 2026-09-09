use {
    anchor_lang::{
        prelude::msg, solana_program::clock::Clock, solana_program::instruction::Instruction,
        solana_program::program_pack::Pack, system_program::ID as SYSTEM_PROGRAM_ID,
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    anchor_spl::{
        associated_token::{self, ID as ASSOCIATED_TOKEN_PROGRAM_ID},
        token::spl_token,
    },
    escrowq32026::state::Escrow,
    litesvm::LiteSVM,
    litesvm_token::{
        spl_token::ID as TOKEN_PROGRAM_ID, CreateAssociatedTokenAccount, CreateMint, MintTo,
    },
    solana_keypair::Keypair,
    solana_message::Message,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_transaction::Transaction,
};

const DECIMALS: u8 = 6;
const INITIAL_BALANCE: u64 = 1_000_000_000;
const ESCROW_SEED_TAKE: u64 = 123;
const ESCROW_SEED_REFUND: u64 = 456;

fn setup() -> (LiteSVM, Keypair) {
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/escrowq32026.so"
    ));

    svm.add_program(escrowq32026::id(), bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();
    (svm, payer)
}

fn send_instruction(
    program: &mut LiteSVM,
    payer: &Keypair,
    label: &str,
    instruction: Instruction,
) {
    let message = Message::new(&[instruction], Some(&payer.pubkey()));
    let transaction = Transaction::new(&[payer], message, program.latest_blockhash());
    let result = program.send_transaction(transaction).unwrap();

    msg!("{} succeeded", label);
    msg!("  signature: {}", result.signature);
    msg!("  compute units: {}", result.compute_units_consumed);
}

fn send_instruction_expect_failure(
    program: &mut LiteSVM,
    payer: &Keypair,
    label: &str,
    instruction: Instruction,
) {
    let message = Message::new(&[instruction], Some(&payer.pubkey()));
    let transaction = Transaction::new(&[payer], message, program.latest_blockhash());
    let result = program.send_transaction(transaction);

    assert!(result.is_err(), "{label} unexpectedly succeeded");
    msg!("{} rejected as expected", label);
}

fn escrow_address(maker: &Pubkey, seed: u64) -> Pubkey {
    Pubkey::find_program_address(
        &[b"escrow", maker.as_ref(), &seed.to_le_bytes()],
        &escrowq32026::id(),
    )
    .0
}

fn vault_address(escrow: &Pubkey, mint: &Pubkey) -> Pubkey {
    associated_token::get_associated_token_address(escrow, mint)
}

fn token_balance(program: &LiteSVM, account: &Pubkey) -> u64 {
    let account = program.get_account(account).unwrap();
    spl_token::state::Account::unpack(&account.data)
        .unwrap()
        .amount
}

fn read_escrow(program: &LiteSVM, escrow: &Pubkey) -> Escrow {
    let account = program.get_account(escrow).unwrap();
    Escrow::try_deserialize(&mut account.data.as_ref()).unwrap()
}

fn make_instruction(
    maker: Pubkey,
    mint_a: Pubkey,
    mint_b: Pubkey,
    maker_ata_a: Pubkey,
    escrow: Pubkey,
    vault: Pubkey,
    seed: u64,
    deposit: u64,
    receive: u64,
    expiration: i64,
) -> Instruction {
    Instruction {
        program_id: escrowq32026::id(),
        accounts: escrowq32026::accounts::Make {
            maker,
            mint_a,
            mint_b,
            maker_ata_a,
            escrow,
            vault,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            token_program: TOKEN_PROGRAM_ID,
            system_program: SYSTEM_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: escrowq32026::instruction::Make {
            seed,
            deposit,
            receive,
            expiration,
        }
        .data(),
    }
}

fn refund_instruction(
    maker: Pubkey,
    mint_a: Pubkey,
    maker_ata_a: Pubkey,
    escrow: Pubkey,
    vault: Pubkey,
) -> Instruction {
    Instruction {
        program_id: escrowq32026::id(),
        accounts: escrowq32026::accounts::Refund {
            maker,
            mint_a,
            maker_ata_a,
            escrow,
            vault,
            token_program: TOKEN_PROGRAM_ID,
            system_program: SYSTEM_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: escrowq32026::instruction::Refund {}.data(),
    }
}

#[test]
fn test_make_update_take_and_refund() {
    let (mut program, maker) = setup();
    let maker_pubkey = maker.pubkey();
    let taker = Keypair::new();
    program.airdrop(&taker.pubkey(), 1_000_000_000).unwrap();

    let mint_a = CreateMint::new(&mut program, &maker)
        .decimals(DECIMALS)
        .authority(&maker_pubkey)
        .send()
        .unwrap();
    let mint_b = CreateMint::new(&mut program, &maker)
        .decimals(DECIMALS)
        .authority(&maker_pubkey)
        .send()
        .unwrap();
    let maker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &maker, &mint_a)
        .owner(&maker_pubkey)
        .send()
        .unwrap();
    let taker_ata_b = CreateAssociatedTokenAccount::new(&mut program, &taker, &mint_b)
        .owner(&taker.pubkey())
        .send()
        .unwrap();

    MintTo::new(&mut program, &maker, &mint_a, &maker_ata_a, INITIAL_BALANCE)
        .send()
        .unwrap();
    MintTo::new(&mut program, &maker, &mint_b, &taker_ata_b, INITIAL_BALANCE)
        .send()
        .unwrap();
    msg!("Created mints and funded maker/taker token accounts");

    let take_escrow = escrow_address(&maker_pubkey, ESCROW_SEED_TAKE);
    let take_vault = vault_address(&take_escrow, &mint_a);
    let take_deposit = 10_000_000;
    let initial_receive = 10_000_000;
    let updated_receive = 20_000_000;
    let take_expiration = program.get_sysvar::<Clock>().unix_timestamp + 100;

    send_instruction(
        &mut program,
        &maker,
        "make (take flow)",
        make_instruction(
            maker_pubkey,
            mint_a,
            mint_b,
            maker_ata_a,
            take_escrow,
            take_vault,
            ESCROW_SEED_TAKE,
            take_deposit,
            initial_receive,
            take_expiration,
        ),
    );
    assert_eq!(token_balance(&program, &take_vault), take_deposit);
    assert_eq!(read_escrow(&program, &take_escrow).receive, initial_receive);

    send_instruction(
        &mut program,
        &maker,
        "update",
        Instruction {
            program_id: escrowq32026::id(),
            accounts: escrowq32026::accounts::Update {
                maker: maker_pubkey,
                escrow: take_escrow,
            }
            .to_account_metas(None),
            data: escrowq32026::instruction::Update {
                seed: ESCROW_SEED_TAKE,
                receive: updated_receive,
                expiration: take_expiration + 100,
            }
            .data(),
        },
    );
    let updated_escrow = read_escrow(&program, &take_escrow);
    assert_eq!(updated_escrow.receive, updated_receive);
    assert_eq!(updated_escrow.expiration, take_expiration + 100);

    let taker_ata_a = associated_token::get_associated_token_address(&taker.pubkey(), &mint_a);
    let maker_ata_b = associated_token::get_associated_token_address(&maker_pubkey, &mint_b);
    let maker_a_before_take = token_balance(&program, &maker_ata_a);
    let taker_b_before_take = token_balance(&program, &taker_ata_b);

    send_instruction(
        &mut program,
        &taker,
        "take",
        Instruction {
            program_id: escrowq32026::id(),
            accounts: escrowq32026::accounts::Take {
                taker: taker.pubkey(),
                maker: maker_pubkey,
                mint_a,
                mint_b,
                taker_ata_a,
                taker_ata_b,
                maker_ata_b,
                escrow: take_escrow,
                vault: take_vault,
                token_program: TOKEN_PROGRAM_ID,
                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: escrowq32026::instruction::Take {}.data(),
        },
    );
    assert_eq!(token_balance(&program, &maker_ata_b), updated_receive);
    assert_eq!(
        token_balance(&program, &taker_ata_b),
        taker_b_before_take - updated_receive
    );
    assert_eq!(token_balance(&program, &maker_ata_a), maker_a_before_take);
    assert_eq!(token_balance(&program, &taker_ata_a), take_deposit);
    assert!(program.get_account(&take_escrow).is_none());
    assert!(program.get_account(&take_vault).is_none());
    msg!("take balances and account closures verified");

    let refund_escrow = escrow_address(&maker_pubkey, ESCROW_SEED_REFUND);
    let refund_vault = vault_address(&refund_escrow, &mint_a);
    let refund_deposit = 30_000_000;
    let maker_a_before_refund = token_balance(&program, &maker_ata_a);
    let refund_expiration = program.get_sysvar::<Clock>().unix_timestamp + 100;

    send_instruction(
        &mut program,
        &maker,
        "make (refund flow)",
        make_instruction(
            maker_pubkey,
            mint_a,
            mint_b,
            maker_ata_a,
            refund_escrow,
            refund_vault,
            ESCROW_SEED_REFUND,
            refund_deposit,
            1,
            refund_expiration,
        ),
    );

    send_instruction_expect_failure(
        &mut program,
        &maker,
        "refund",
        refund_instruction(
            maker_pubkey,
            mint_a,
            maker_ata_a,
            refund_escrow,
            refund_vault,
        ),
    );

    let mut clock = program.get_sysvar::<Clock>();
    clock.unix_timestamp = refund_expiration;
    program.set_sysvar(&clock);
    program.expire_blockhash();
    msg!("Advanced clock to escrow expiration: {}", clock.unix_timestamp);

    send_instruction(
        &mut program,
        &maker,
        "refund after expiration",
        refund_instruction(
            maker_pubkey,
            mint_a,
            maker_ata_a,
            refund_escrow,
            refund_vault,
        ),
    );
    assert_eq!(token_balance(&program, &maker_ata_a), maker_a_before_refund);
    assert!(program.get_account(&refund_escrow).is_none());
    assert!(program.get_account(&refund_vault).is_none());
    msg!("refund balance and account closures verified");
}
