#![allow(non_snake_case)]
#![no_std]
use soroban_sdk::{contract, contracttype, contractimpl, log, Env, Address, String, Vec};

// Structure to define NFT details
#[contracttype]
#[derive(Clone)]
pub struct NFT {
    pub id: u64,
    pub owner: Address,
    pub name: String,
    pub attributes: Vec<String>,
    pub level: u32,
    pub created_at: u64,
}

// Structure for defining merge recipes
#[contracttype]
#[derive(Clone)]
pub struct MergeRecipe {
    pub id: u64,
    pub required_nft_types: Vec<String>,
    pub result_name: String,
    pub result_attributes: Vec<String>,
    pub level_bonus: u32,
}

// Data keys for storage
#[contracttype]
pub enum DataKey {
    NFT(u64),          // NFT ID -> NFT
    NFTCounter,        // Counter for generating unique NFT IDs
    Recipe(u64),       // Recipe ID -> MergeRecipe
    RecipeCounter,     // Counter for generating unique recipe IDs
    OwnerNFTs(Address) // Address -> Vec of owned NFT IDs
}

#[contract]
pub struct NFTMergeContract;

#[contractimpl]
impl NFTMergeContract {
    // Create a new NFT
    pub fn mint_nft(
        env: Env,
        owner: Address,
        name: String,
        attributes: Vec<String>,
        level: u32
    ) -> u64 {
        // Verify the owner
        owner.require_auth();
        
        // Get the next NFT ID
        let nft_counter: u64 = env.storage().instance().get(&DataKey::NFTCounter).unwrap_or(0);
        let nft_id = nft_counter + 1;
        
        // Create new NFT
        let nft = NFT {
            id: nft_id,
            owner: owner.clone(),
            name,
            attributes,
            level,
            created_at: env.ledger().timestamp(),
        };
        
        // Store the NFT
        env.storage().instance().set(&DataKey::NFT(nft_id), &nft);
        
        // Update the counter
        env.storage().instance().set(&DataKey::NFTCounter, &nft_id);
        
        // Update owner's NFT list
        let owner_key = DataKey::OwnerNFTs(owner.clone());
        let mut owner_nfts: Vec<u64> = env.storage().instance().get(&owner_key).unwrap_or(Vec::new(&env));
        owner_nfts.push_back(nft_id);
        env.storage().instance().set(&owner_key, &owner_nfts);
        
        // Extend contract data TTL
        env.storage().instance().extend_ttl(100, 100);
        
        log!(&env, "NFT minted with ID: {}", nft_id);
        nft_id
    }
    
    // Create a new merge recipe
    pub fn create_recipe(
        env: Env,
        admin: Address,
        required_nft_types: Vec<String>,
        result_name: String,
        result_attributes: Vec<String>,
        level_bonus: u32
    ) -> u64 {
        // Verify the admin
        admin.require_auth();
        
        // Get the next recipe ID
        let recipe_counter: u64 = env.storage().instance().get(&DataKey::RecipeCounter).unwrap_or(0);
        let recipe_id = recipe_counter + 1;
        
        // Create new recipe
        let recipe = MergeRecipe {
            id: recipe_id,
            required_nft_types,
            result_name,
            result_attributes,
            level_bonus,
        };
        
        // Store the recipe
        env.storage().instance().set(&DataKey::Recipe(recipe_id), &recipe);
        
        // Update the counter
        env.storage().instance().set(&DataKey::RecipeCounter, &recipe_id);
        
        // Extend contract data TTL
        env.storage().instance().extend_ttl(100, 100);
        
        log!(&env, "Recipe created with ID: {}", recipe_id);
        recipe_id
    }
    
    // Merge NFTs according to recipe
    pub fn merge_nfts(
        env: Env,
        owner: Address,
        nft_ids: Vec<u64>,
        recipe_id: u64
    ) -> u64 {
        // Verify the owner
        owner.require_auth();
        
        // Check if we have enough NFTs for merging
        if nft_ids.len() < 2 {
            log!(&env, "Need at least 2 NFTs to merge");
            panic!("Need at least 2 NFTs to merge");
        }
        
        // Get the recipe
        let recipe: MergeRecipe = match env.storage().instance().get(&DataKey::Recipe(recipe_id)) {
            Some(r) => r,
            None => {
                log!(&env, "Recipe does not exist");
                panic!("Recipe does not exist");
            }
        };
        
        // Verify NFT ownership and collect NFTs for merging
        let mut nfts_to_merge: Vec<NFT> = Vec::new(&env);
        let mut cumulative_level: u32 = 0;
        
        for i in 0..nft_ids.len() {
            let nft_id = nft_ids.get(i).unwrap();
            let nft: NFT = match env.storage().instance().get(&DataKey::NFT(nft_id)) {
                Some(n) => n,
                None => {
                    log!(&env, "NFT {} does not exist", nft_id);
                    panic!("NFT does not exist");
                }
            };
            
            // Check ownership
            if nft.owner != owner {
                log!(&env, "You don't own NFT {}", nft_id);
                panic!("You don't own this NFT");
            }
            
            nfts_to_merge.push_back(nft.clone());
            cumulative_level += nft.level;
        }
        
        // Check if NFTs match the recipe requirements
        // In a real implementation, this would be more complex
        // For simplicity, we're just checking the count here
        if nfts_to_merge.len() != recipe.required_nft_types.len() {
            log!(&env, "NFTs don't match recipe requirements");
            panic!("NFTs don't match recipe requirements");
        }
        
        // Calculate new NFT level (average + bonus)
        let new_level = (cumulative_level / nfts_to_merge.len() as u32) + recipe.level_bonus;
        
        // Mint new NFT
        let new_nft_id = Self::mint_nft(
            env.clone(),
            owner.clone(),
            recipe.result_name.clone(),
            recipe.result_attributes.clone(),
            new_level
        );
        
        // Remove the source NFTs
        let owner_key = DataKey::OwnerNFTs(owner.clone());
        let mut owner_nfts: Vec<u64> = env.storage().instance().get(&owner_key).unwrap_or(Vec::new(&env));
        
        for i in 0..nft_ids.len() {
            let nft_id = nft_ids.get(i).unwrap();
            
            // Remove from owner's list
            let mut new_owner_nfts = Vec::new(&env);
            for j in 0..owner_nfts.len() {
                let id = owner_nfts.get(j).unwrap();
                if id != nft_id {
                    new_owner_nfts.push_back(id);
                }
            }
            owner_nfts = new_owner_nfts;
            
            // Delete the NFT
            env.storage().instance().remove(&DataKey::NFT(nft_id));
        }
        
        // Update owner's NFT list
        env.storage().instance().set(&owner_key, &owner_nfts);
        
        log!(&env, "NFTs merged successfully, new NFT ID: {}", new_nft_id);
        new_nft_id
    }
    
    // View NFT details
    pub fn view_nft(env: Env, nft_id: u64) -> NFT {
        match env.storage().instance().get(&DataKey::NFT(nft_id)) {
            Some(nft) => nft,
            None => {
                log!(&env, "NFT does not exist");
                panic!("NFT does not exist");
            }
        }
    }
}