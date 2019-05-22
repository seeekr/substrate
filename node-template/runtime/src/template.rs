use support::{decl_storage, decl_module, StorageValue, StorageMap,
    dispatch::Result, ensure, decl_event, traits::Currency};
use system::ensure_signed;
use runtime_primitives::traits::{As, Hash, Zero};
use parity_codec::{Encode, Decode};
use rstd::cmp;

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Kitty<Hash, Balance> {
    id: Hash,
    dna: Hash,
    price: Balance,
    gen: u64,
}

pub trait Trait: balances::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as system::Trait>::AccountId,
        <T as system::Trait>::Hash,
        <T as balances::Trait>::Balance
    {
        Created(AccountId, Hash),
        PriceSet(AccountId, Hash, Balance),
        Transferred(AccountId, AccountId, Hash),
        Bought(AccountId, AccountId, Hash, Balance),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as KittyStorage {

        // Step 2. mark something as part of genesis config
        Kitties get(kitty) build(|config: &GenesisConfig<T>| {
            config.kitties.iter()
                .map(|i| (
                    i.1,
                    Kitty {
                        id: i.1,
                        dna: i.1,
                        price: i.2,
                        gen: 0,
                    }
                ))
                .collect::<Vec<_>>()
        }): map T::Hash => Kitty<T::Hash, T::Balance>;

        KittyOwner get(owner_of) build(|config: &GenesisConfig<T>| {
            config.kitties.iter().cloned() //address this
                .map(|i| (i.1, i.0) )
                .collect::<Vec<_>>()
        }): map T::Hash => Option<T::AccountId>;

        AllKittiesArray get(kitty_by_index) build(|config: &GenesisConfig<T>| {
            config.kitties.iter()
                .enumerate()
                .map(|(i, k)| (i as u64, k.1) )
                .collect::<Vec<_>>()
        }): map u64 => T::Hash;

        AllKittiesCount get(all_kitties_count) build(|config: &GenesisConfig<T>| {
            config.kitties.len() as u64
        }): u64;
        
        AllKittiesIndex: map T::Hash => u64;

        // TODO
        OwnedKittiesArray get(kitty_of_owner_by_index): map (T::AccountId, u64) => T::Hash;
        OwnedKittiesCount get(owned_kitty_count): map T::AccountId => u64;
        OwnedKittiesIndex: map T::Hash => u64;

        Nonce: u64;
    }

    // 1. add config 
    add_extra_genesis {
        // accessible as config.kitties, randomly create kitties
        // (0, num of kitties) then call create_kitty(account(num)) to do autominting

        // expect user to provide
        // account which owns the kitty
        // hash: kitty dna (also kitty_id)
        // balance: the price of the kitty
        config(kitties): Vec<(T::AccountId, T::Hash, T::Balance)>;
	}
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {

        fn deposit_event<T>() = default;

        fn create_kitty(origin) -> Result {
            let sender = ensure_signed(origin)?;
            let nonce = <Nonce<T>>::get();
            let random_hash = (<system::Module<T>>::random_seed(), &sender, nonce)
                .using_encoded(<T as system::Trait>::Hashing::hash);

            let new_kitty = Kitty {
                id: random_hash,
                dna: random_hash,
                price: <T::Balance as As<u64>>::sa(0),
                gen: 0,
            };

            Self::mint(sender, random_hash, new_kitty)?;

            <Nonce<T>>::mutate(|n| *n += 1);

            Ok(())
        }

        fn set_price(origin, kitty_id: T::Hash, new_price: T::Balance) -> Result {
            let sender = ensure_signed(origin)?;

            ensure!(<Kitties<T>>::exists(kitty_id), "This cat does not exist");

            let owner = Self::owner_of(kitty_id).ok_or("No owner for this kitty")?;
            ensure!(owner == sender, "You do not own this cat");

            let mut kitty = Self::kitty(kitty_id);
            kitty.price = new_price;

            <Kitties<T>>::insert(kitty_id, kitty);

            Self::deposit_event(RawEvent::PriceSet(sender, kitty_id, new_price));

            Ok(())
        }

        fn transfer(origin, to: T::AccountId, kitty_id: T::Hash) -> Result {
            let sender = ensure_signed(origin)?;

            let owner = Self::owner_of(kitty_id).ok_or("No owner for this kitty")?;
            ensure!(owner == sender, "You do not own this kitty");

            Self::transfer_from(sender, to, kitty_id)?;

            Ok(())
        }

        fn buy_kitty(origin, kitty_id: T::Hash, max_price: T::Balance) -> Result {
            let sender = ensure_signed(origin)?;

            ensure!(<Kitties<T>>::exists(kitty_id), "This cat does not exist");

            let owner = Self::owner_of(kitty_id).ok_or("No owner for this kitty")?;
            ensure!(owner != sender, "You can't buy your own cat");

            let mut kitty = Self::kitty(kitty_id);

            let kitty_price = kitty.price;
            ensure!(!kitty_price.is_zero(), "The cat you want to buy is not for sale");
            ensure!(kitty_price <= max_price, "The cat you want to buy costs more than your max price");

            <balances::Module<T> as Currency<_>>::transfer(&sender, &owner, kitty_price)?;

            Self::transfer_from(owner.clone(), sender.clone(), kitty_id)
                .expect("`owner` is shown to own the kitty; \
                `owner` must have greater than 0 kitties, so transfer cannot cause underflow; \
                `all_kitty_count` shares the same type as `owned_kitty_count` \
                and minting ensure there won't ever be more than `max()` kitties, \
                which means transfer cannot cause an overflow; \
                qed");

            kitty.price = <T::Balance as As<u64>>::sa(0);
            <Kitties<T>>::insert(kitty_id, kitty);

            Self::deposit_event(RawEvent::Bought(sender, owner, kitty_id, kitty_price));

            Ok(())
        }

        fn breed_kitty(origin, kitty_id_1: T::Hash, kitty_id_2: T::Hash) -> Result{
            let sender = ensure_signed(origin)?;

            ensure!(<Kitties<T>>::exists(kitty_id_1), "This cat 1 does not exist");
            ensure!(<Kitties<T>>::exists(kitty_id_2), "This cat 2 does not exist");

            let nonce = <Nonce<T>>::get();
            let random_hash = (<system::Module<T>>::random_seed(), &sender, nonce)
                .using_encoded(<T as system::Trait>::Hashing::hash);

            let kitty_1 = Self::kitty(kitty_id_1);
            let kitty_2 = Self::kitty(kitty_id_2);

            let mut final_dna = kitty_1.dna;
            for (i, (dna_2_element, r)) in kitty_2.dna.as_ref().iter().zip(random_hash.as_ref().iter()).enumerate() {
                if r % 2 == 0 {
                    final_dna.as_mut()[i] = *dna_2_element;
                }
            }

            let new_kitty = Kitty {
                id: random_hash,
                dna: final_dna,
                price: <T::Balance as As<u64>>::sa(0),
                gen: cmp::max(kitty_1.gen, kitty_2.gen) + 1,
            };

            Self::mint(sender, random_hash, new_kitty)?;

            <Nonce<T>>::mutate(|n| *n += 1);

            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    fn mint(to: T::AccountId, kitty_id: T::Hash, new_kitty: Kitty<T::Hash, T::Balance>) -> Result {
        ensure!(!<KittyOwner<T>>::exists(kitty_id), "Kitty already exists");

        let owned_kitty_count = Self::owned_kitty_count(&to);

        let new_owned_kitty_count = owned_kitty_count.checked_add(1)
            .ok_or("Overflow adding a new kitty to account balance")?;

        let all_kitties_count = Self::all_kitties_count();

        let new_all_kitties_count = all_kitties_count.checked_add(1)
            .ok_or("Overflow adding a new kitty to total supply")?;

        <Kitties<T>>::insert(kitty_id, new_kitty);
        <KittyOwner<T>>::insert(kitty_id, &to);

        <AllKittiesArray<T>>::insert(all_kitties_count, kitty_id);
        <AllKittiesCount<T>>::put(new_all_kitties_count);
        <AllKittiesIndex<T>>::insert(kitty_id, all_kitties_count);

        <OwnedKittiesArray<T>>::insert((to.clone(), owned_kitty_count), kitty_id);
        <OwnedKittiesCount<T>>::insert(&to, new_owned_kitty_count);
        <OwnedKittiesIndex<T>>::insert(kitty_id, owned_kitty_count);

        Self::deposit_event(RawEvent::Created(to, kitty_id));

        Ok(())
    }

    fn transfer_from(from: T::AccountId, to: T::AccountId, kitty_id: T::Hash) -> Result {
        let owner = Self::owner_of(kitty_id).ok_or("No owner for this kitty")?;

        ensure!(owner == from, "'from' account does not own this kitty");

        let owned_kitty_count_from = Self::owned_kitty_count(&from);
        let owned_kitty_count_to = Self::owned_kitty_count(&to);

        let new_owned_kitty_count_to = owned_kitty_count_to.checked_add(1)
            .ok_or("Transfer causes overflow of 'to' kitty balance")?;

        let new_owned_kitty_count_from = owned_kitty_count_from.checked_sub(1)
            .ok_or("Transfer causes underflow of 'from' kitty balance")?;

        let kitty_index = <OwnedKittiesIndex<T>>::get(kitty_id);
        if kitty_index != new_owned_kitty_count_from {
            let last_kitty_id = <OwnedKittiesArray<T>>::get((from.clone(), new_owned_kitty_count_from));
            <OwnedKittiesArray<T>>::insert((from.clone(), kitty_index), last_kitty_id);
            <OwnedKittiesIndex<T>>::insert(last_kitty_id, kitty_index);
        }

        <KittyOwner<T>>::insert(&kitty_id, &to);
        <OwnedKittiesIndex<T>>::insert(kitty_id, owned_kitty_count_to);

        <OwnedKittiesArray<T>>::remove((from.clone(), new_owned_kitty_count_from));
        <OwnedKittiesArray<T>>::insert((to.clone(), owned_kitty_count_to), kitty_id);

        <OwnedKittiesCount<T>>::insert(&from, new_owned_kitty_count_from);
        <OwnedKittiesCount<T>>::insert(&to, new_owned_kitty_count_to);

        Self::deposit_event(RawEvent::Transferred(from, to, kitty_id));

        Ok(())
    }
}


// TODO: we probably want to move this to a test.rs to keep the code in the sidebar small. OR not? in favor of having the source and test close to each other?
#[cfg(test)]
mod tests {
    use super::*;
	// Import a bunch of things from substrate core. All needed for some parts of the code.
	use support::{impl_outer_origin, assert_ok, assert_noop};
	use runtime_io::{with_externalities, TestExternalities};
	use primitives::{H256, Blake2Hasher};
	use runtime_primitives::{
		BuildStorage, traits::{BlakeTwo256, IdentityLookup},
		testing::{Digest, DigestItem, Header}
	};

	// TODO: learn this. wtf does it exactly do?
	impl_outer_origin! {
		pub enum Origin for KittiesTest {}
	}

	// Create one empty type for testing.
	#[derive(Clone, Eq, PartialEq)]
	pub struct KittiesTest;


	// Start implementing the Trait's of all the other modules that you need.
	// If you want anything that is reasonably functional you also need to implement the System trait.
	// TODO: justify why system is needed (bc I have seen tests without it. It is for sure not mandatory.)
	impl system::Trait for KittiesTest {
		type Origin = Origin;
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type Digest = Digest;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = ();
		type Log = DigestItem;
	}
	// And any other trait that your Trait is explicitly bounded by.
	// Remember you had: `pub trait Trait: balances::Trait`
	impl balances::Trait for KittiesTest {
		type Balance = u64;
		type OnFreeBalanceZero = ();
		type OnNewAccount = ();
		type Event = ();
		type TransactionPayment = ();
		type TransferPayment = ();
		type DustRemoval = ();
	}
	// And finally, your own trait.
	impl super::Trait for KittiesTest {
		type Event = ();
	}

	// To make any fo the modules more accessible easier:
	// You can do this for any of the modules, for which you've implemented its Trait for `KittiesTest`.
	// TODO: explain why module is what we need.
	type Kitties = super::Module<KittiesTest>;
	type Balances = balances::Module<KittiesTest>;
	type System = system::Module<KittiesTest>;


    // Vec<(T::AccountId, T::Hash, T::Balance)>;
    // Step 3: 
	fn build_ext() -> TestExternalities<Blake2Hasher> {
		let mut t = system::GenesisConfig::<KittiesTest>::default().build_storage().unwrap().0;
		t.extend(balances::GenesisConfig::<KittiesTest>::default().build_storage().unwrap().0);
		t.extend(GenesisConfig::<KittiesTest> { // 3. new stuff here
            kitties: vec![  (0, H256::random(), 50), 
                            (1, H256::zero(), 100)], 
            ..Default::default() // Do i need this?
        }.build_storage().unwrap().0);
		t.into()
	}

	#[test]
    #[ignore]
	fn create_kitty_should_work() {
		with_externalities(&mut build_ext(), || {
			// create a kitty with account 10.
			// TODO: generally: I don't know enough about origin to rant about it.
			assert_ok!(Kitties::create_kitty(Origin::signed(10)));

			// check (some) storage items.
			assert_eq!(Kitties::all_kitties_count(), 1);
			assert_eq!(Kitties::owned_kitty_count(10), 1);
			// no one else has a kitty, only 10
			assert_eq!(Kitties::owned_kitty_count(5), 0);

			let hash = Kitties::kitty_by_index(0);
			assert_eq!(Kitties::owner_of(hash), Some(10));

			let other_hash = Kitties::kitty_of_owner_by_index((10, 0));
			assert_eq!(hash, other_hash);

			// alternative syntax:
			use super::KittyOwner;
			use support::StorageMap;
			assert_eq!(<KittyOwner<KittiesTest>>::get(hash), Some(10));
		})
	}

	#[test]
    #[ignore]
	fn transfer_kitty_should_work() {
		with_externalities(&mut build_ext(), || {
			// check that 10 own a kitty
			assert_ok!(Kitties::create_kitty(Origin::signed(10)));

			assert_eq!(Kitties::owned_kitty_count(10), 1);
			let hash = Kitties::kitty_of_owner_by_index((10, 0));

			// send kitty to 1.
			assert_ok!(Kitties::transfer(Origin::signed(10), 1, hash));

			// 10 now has nothing
			assert_eq!(Kitties::owned_kitty_count(10), 0);
			// but 1 does
			assert_eq!(Kitties::owned_kitty_count(1), 1);
			let new_hash = Kitties::kitty_of_owner_by_index((1, 0));
			// and it has the same hash
			assert_eq!(hash, new_hash);
		})
	}

	#[test]
    #[ignore]
	fn transfer_not_owned_kitty_should_fail() {
		with_externalities(&mut build_ext(), || {
			// check that 10 own a kitty
			assert_ok!(Kitties::create_kitty(Origin::signed(10)));
			let hash = Kitties::kitty_of_owner_by_index((10, 0));

			// account 0 cannot transfer a kitty with this hash.
			assert_noop!(Kitties::transfer(Origin::signed(9), 1, hash), "You do not own this kitty");
		})
	}


// (0, H256::random(), 50), (1, H256::zero(), 100)], 
    // Step 4
    #[test]
    fn should_build_genesis_kitties() {
        with_externalities(&mut build_ext(), || {
            // Check that 2nd kitty exists at genesis, with value 100
            assert_eq!(Kitties::kitty(H256::zero()).price, 100);
            
            assert_eq!(Kitties::owner_of(H256::zero()), Some(1));

            let kitty0 = Kitties::kitty_by_index(0);

            assert_ne!(kitty0, H256::zero());
            assert_eq!(Kitties::kitty_by_index(1), H256::zero());

            assert_eq!(Kitties::all_kitties_count(), 2);

        })
    }
}