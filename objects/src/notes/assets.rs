use core::cell::OnceCell;

use super::{
    Asset, ByteReader, ByteWriter, Deserializable, DeserializationError, Digest, Felt, Hasher,
    NoteError, Serializable, Vec, Word, WORD_SIZE, ZERO,
};

// NOTE ASSETS
// ================================================================================================
/// An asset container for a note.
///
/// A note can contain up to 255 assets. No duplicates are allowed, but the order of assets is
/// unspecified.
///
/// All the assets in a note can be reduced to a single commitment which is computed by
/// sequentially hashing the assets. Note that the same list of assets can result in two different
/// commitments if the asset ordering is different.
#[derive(Debug, Clone)]
pub struct NoteAssets {
    assets: Vec<Asset>,
    hash: OnceCell<Digest>,
}

impl NoteAssets {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------
    /// The maximum number of assets which can be carried by a single note.
    pub const MAX_NUM_ASSETS: usize = 255;

    // CONSTRUCTOR
    // --------------------------------------------------------------------------------------------
    /// Returns new [NoteAssets] constructed from the provided list of assets.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The asset list is empty.
    /// - The list contains more than 255 assets.
    /// - There are duplicate assets in the list.
    pub fn new(assets: &[Asset]) -> Result<Self, NoteError> {
        if assets.is_empty() {
            return Err(NoteError::EmptyAssetList);
        } else if assets.len() > Self::MAX_NUM_ASSETS {
            return Err(NoteError::too_many_assets(assets.len()));
        }

        // make sure all provided assets are unique
        for (i, asset) in assets.iter().enumerate() {
            // for all assets except the last one, check if the asset is the same as any other
            // asset in the list, and if so return an error
            if i < assets.len() - 1 && assets[i + 1..].iter().any(|a| a.is_same(asset)) {
                return Err(match asset {
                    Asset::Fungible(a) => NoteError::duplicate_fungible_asset(a.faucet_id()),
                    Asset::NonFungible(a) => NoteError::duplicate_non_fungible_asset(*a),
                });
            }
        }

        Ok(Self {
            assets: assets.to_vec(),
            hash: OnceCell::new(),
        })
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns a commitment to the note's assets.
    pub fn commitment(&self) -> Digest {
        *self.hash.get_or_init(|| compute_asset_commitment(&self.assets))
    }

    /// Returns the number of assets.
    pub fn num_assets(&self) -> usize {
        self.assets.len()
    }

    /// Returns an iterator over all assets.
    pub fn iter(&self) -> core::slice::Iter<Asset> {
        self.assets.iter()
    }

    /// Returns all assets represented as a vector of field elements.
    ///
    /// The vector is padded with ZEROs so that its length is a multiple of 8. This is useful
    /// because hashing the returned elements results in the note asset commitment.
    pub fn to_padded_assets(&self) -> Vec<Felt> {
        // if we have an odd number of assets with pad with a single word.
        let padded_len = if self.assets.len() % 2 == 0 {
            self.assets.len() * WORD_SIZE
        } else {
            (self.assets.len() + 1) * WORD_SIZE
        };

        // allocate a vector to hold the padded assets
        let mut padded_assets = Vec::with_capacity(padded_len * WORD_SIZE);

        // populate the vector with the assets
        padded_assets.extend(self.assets.iter().flat_map(|asset| <[Felt; 4]>::from(*asset)));

        // pad with an empty word if we have an odd number of assets
        padded_assets.resize(padded_len, ZERO);

        padded_assets
    }
}

impl PartialEq for NoteAssets {
    fn eq(&self, other: &Self) -> bool {
        self.assets == other.assets
    }
}

impl Eq for NoteAssets {}

// HELPER FUNCTIONS
// ================================================================================================

/// Returns a commitment to a note's assets.
///
/// The commitment is computed as a sequential hash of all assets (each asset represented by 4
/// field elements), padded to the next multiple of 2.
fn compute_asset_commitment(assets: &[Asset]) -> Digest {
    // If we have an odd number of assets we pad the vector with 4 zero elements. This is to
    // ensure the number of elements is a multiple of 8 - the size of the hasher rate.
    let word_capacity = if assets.len() % 2 == 0 {
        assets.len()
    } else {
        assets.len() + 1
    };
    let mut asset_elements = Vec::with_capacity(word_capacity * WORD_SIZE);

    for asset in assets.iter() {
        // convert the asset into field elements and add them to the list elements
        let asset_word: Word = (*asset).into();
        asset_elements.extend_from_slice(&asset_word);
    }

    // If we have an odd number of assets we pad the vector with 4 zero elements. This is to
    // ensure the number of elements is a multiple of 8 - the size of the hasher rate. This
    // simplifies hashing inside of the virtual machine when ingesting assets from a note.
    if assets.len() % 2 == 1 {
        asset_elements.extend_from_slice(&Word::default());
    }

    Hasher::hash_elements(&asset_elements)
}

// SERIALIZATION
// ================================================================================================

impl Serializable for NoteAssets {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        debug_assert!(self.assets.len() <= NoteAssets::MAX_NUM_ASSETS);
        target.write_u8((self.assets.len() - 1) as u8);
        self.assets.write_into(target);
    }
}

impl Deserializable for NoteAssets {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let count = source.read_u8()? + 1;
        let assets = Asset::read_batch_from(source, count.into())?;

        Self::new(&assets).map_err(|e| DeserializationError::InvalidValue(format!("{e:?}")))
    }
}
