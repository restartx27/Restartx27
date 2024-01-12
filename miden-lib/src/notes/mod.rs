use miden_objects::{
    accounts::AccountId, assets::Asset, crypto::rand::FeltRng, notes::Note,
    utils::collections::Vec, Felt, NoteError, Word, ZERO,
};

use self::utils::build_note_script;

pub mod utils;

// STANDARDIZED SCRIPTS
// ================================================================================================

/// Generates a P2ID note - pay to id note.
/// This script enables the transfer of assets from one account `sender` to another account`target`
pub fn create_p2id_note<R: FeltRng>(
    sender: AccountId,
    target: AccountId,
    assets: Vec<Asset>,
    mut rng: R,
) -> Result<Note, NoteError> {
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/note_scripts/P2ID.masb"));
    let note_script = build_note_script(bytes)?;

    let inputs = vec![target.into(), ZERO, ZERO, ZERO];
    let tag: Felt = target.into();
    let serial_num = rng.draw_word();

    Note::new(note_script, &inputs, &assets, serial_num, sender, tag)
}

/// Generates a P2IDR note - pay to id with recall after a certain block height.
/// This script enables the transfer of assets from one account `sender` to another account `target`
/// additionally it adds the possibility of a recall window enabling reclaiming of assets if the
/// note has not been consumed by the `target` in the inputed timeframe
pub fn create_p2idr_note<R: FeltRng>(
    sender: AccountId,
    target: AccountId,
    assets: Vec<Asset>,
    recall_height: u32,
    mut rng: R,
) -> Result<Note, NoteError> {
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/note_scripts/P2IDR.masb"));
    let note_script = build_note_script(bytes)?;

    let inputs = vec![target.into(), recall_height.into(), ZERO, ZERO];
    let tag: Felt = target.into();
    let serial_num = rng.draw_word();

    Note::new(note_script.clone(), &inputs, &assets, serial_num, sender, tag)
}

/// Generates a SWAP note - swap of assets between two accounts.
/// This script enables a swap of 2 assets between one account `sender` and any other account that
/// is willing to consume the note. The consumer will receive the `offered_asset` and will create a
/// new P2ID note with `sender` as target, containing the `requested_asset`
pub fn create_swap_note<R: FeltRng>(
    sender: AccountId,
    offered_asset: Asset,
    requested_asset: Asset,
    mut rng: R,
) -> Result<(Note, Word), NoteError> {
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/assets/note_scripts/SWAP.masb"));
    let note_script = build_note_script(bytes)?;

    let repay_serial_num = rng.draw_word();
    let recipient = utils::build_p2id_recipient(sender, repay_serial_num)?;
    let asset_word: Word = requested_asset.into();

    let inputs = vec![
        recipient[0],
        recipient[1],
        recipient[2],
        recipient[3],
        asset_word[0],
        asset_word[1],
        asset_word[2],
        asset_word[3],
        sender.into(),
        ZERO,
        ZERO,
        ZERO,
    ];

    let tag: Felt = Felt::new(0);
    let serial_num = rng.draw_word();

    let note = Note::new(note_script.clone(), &inputs, &[offered_asset], serial_num, sender, tag)?;

    Ok((note, repay_serial_num))
}
