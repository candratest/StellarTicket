#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env, String, Symbol};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event {
    pub organizer: Address,
    pub name: String,
    pub description: String,
    pub location: String,
    pub event_date: u64,
    pub ticket_price: i128,
    pub max_tickets: u32,
    pub sold_tickets: u32,
    pub active: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ticket {
    pub ticket_id: Symbol,
    pub event_id: Symbol,
    pub owner: Address,
    pub issued_at: u64,
    pub used: bool,
    pub valid: bool,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Event(Symbol),
    Ticket(Symbol),
    UserTicket(Address, Symbol),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    EventAlreadyExists = 3,
    EventNotFound = 4,
    TicketAlreadyExists = 5,
    TicketNotFound = 6,
    EventInactive = 7,
    EventDateMustBeFuture = 8,
    MaxTicketsMustBePositive = 9,
    TicketPriceMustBeNonNegative = 10,
    SoldOut = 11,
    UserAlreadyHasTicket = 12,
    TicketInvalid = 13,
    TicketAlreadyUsed = 14,
    Unauthorized = 15,
    NotTicketOwner = 16,
}

#[contract]
pub struct StellarTicket;

#[contractimpl]
impl StellarTicket {
    pub fn init_admin(env: Env, admin: Address) {
        admin.require_auth();

        if env.storage().persistent().has(&DataKey::Admin) {
            panic_with_error(&env, Error::AlreadyInitialized);
        }

        env.storage().persistent().set(&DataKey::Admin, &admin);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_event(
        env: Env,
        event_id: Symbol,
        organizer: Address,
        name: String,
        description: String,
        location: String,
        event_date: u64,
        ticket_price: i128,
        max_tickets: u32,
    ) {
        organizer.require_auth();

        let event_key = DataKey::Event(event_id.clone());
        if env.storage().persistent().has(&event_key) {
            panic_with_error(&env, Error::EventAlreadyExists);
        }

        if event_date <= env.ledger().timestamp() {
            panic_with_error(&env, Error::EventDateMustBeFuture);
        }

        if max_tickets == 0 {
            panic_with_error(&env, Error::MaxTicketsMustBePositive);
        }

        if ticket_price < 0 {
            panic_with_error(&env, Error::TicketPriceMustBeNonNegative);
        }

        let event = Event {
            organizer,
            name,
            description,
            location,
            event_date,
            ticket_price,
            max_tickets,
            sold_tickets: 0,
            active: true,
        };

        env.storage().persistent().set(&event_key, &event);
    }

    pub fn buy_ticket(env: Env, event_id: Symbol, ticket_id: Symbol, buyer: Address) {
        buyer.require_auth();

        let event_key = DataKey::Event(event_id.clone());
        let mut event = get_event_or_panic(&env, &event_key);

        if !event.active {
            panic_with_error(&env, Error::EventInactive);
        }

        if event.sold_tickets >= event.max_tickets {
            panic_with_error(&env, Error::SoldOut);
        }

        let ticket_key = DataKey::Ticket(ticket_id.clone());
        if env.storage().persistent().has(&ticket_key) {
            panic_with_error(&env, Error::TicketAlreadyExists);
        }

        let user_ticket_key = DataKey::UserTicket(buyer.clone(), event_id.clone());
        if env.storage().persistent().has(&user_ticket_key) {
            panic_with_error(&env, Error::UserAlreadyHasTicket);
        }

        let ticket = Ticket {
            ticket_id: ticket_id.clone(),
            event_id,
            owner: buyer,
            issued_at: env.ledger().timestamp(),
            used: false,
            valid: true,
        };

        event.sold_tickets += 1;

        env.storage().persistent().set(&event_key, &event);
        env.storage().persistent().set(&ticket_key, &ticket);
        env.storage().persistent().set(&user_ticket_key, &ticket_id);
    }

    pub fn get_event(env: Env, event_id: Symbol) -> Event {
        get_event_or_panic(&env, &DataKey::Event(event_id))
    }

    pub fn get_ticket(env: Env, ticket_id: Symbol) -> Ticket {
        get_ticket_or_panic(&env, &DataKey::Ticket(ticket_id))
    }

    pub fn verify_ticket(env: Env, ticket_id: Symbol) -> bool {
        let ticket_key = DataKey::Ticket(ticket_id);
        let ticket = env.storage().persistent().get::<_, Ticket>(&ticket_key);

        match ticket {
            Some(ticket) => {
                if !ticket.valid || ticket.used {
                    return false;
                }

                let event = env
                    .storage()
                    .persistent()
                    .get::<_, Event>(&DataKey::Event(ticket.event_id));

                match event {
                    Some(event) => event.active,
                    None => false,
                }
            }
            None => false,
        }
    }

    pub fn use_ticket(env: Env, ticket_id: Symbol, checker: Address) {
        checker.require_auth();

        let ticket_key = DataKey::Ticket(ticket_id);
        let mut ticket = get_ticket_or_panic(&env, &ticket_key);

        if !ticket.valid {
            panic_with_error(&env, Error::TicketInvalid);
        }

        if ticket.used {
            panic_with_error(&env, Error::TicketAlreadyUsed);
        }

        let event = get_event_or_panic(&env, &DataKey::Event(ticket.event_id.clone()));
        if !event.active {
            panic_with_error(&env, Error::EventInactive);
        }

        require_organizer_or_admin(&env, &checker, &event.organizer);

        ticket.used = true;
        env.storage().persistent().set(&ticket_key, &ticket);
    }

    pub fn transfer_ticket(env: Env, ticket_id: Symbol, from: Address, to: Address) {
        from.require_auth();

        let ticket_key = DataKey::Ticket(ticket_id);
        let mut ticket = get_ticket_or_panic(&env, &ticket_key);

        if ticket.owner != from {
            panic_with_error(&env, Error::NotTicketOwner);
        }

        if ticket.used {
            panic_with_error(&env, Error::TicketAlreadyUsed);
        }

        if !ticket.valid {
            panic_with_error(&env, Error::TicketInvalid);
        }

        let from_ticket_key = DataKey::UserTicket(from.clone(), ticket.event_id.clone());
        let to_ticket_key = DataKey::UserTicket(to.clone(), ticket.event_id.clone());

        if env.storage().persistent().has(&to_ticket_key) {
            panic_with_error(&env, Error::UserAlreadyHasTicket);
        }

        ticket.owner = to;

        env.storage().persistent().remove(&from_ticket_key);
        env.storage().persistent().set(&to_ticket_key, &ticket.ticket_id);
        env.storage().persistent().set(&ticket_key, &ticket);
    }

    pub fn cancel_event(env: Env, event_id: Symbol, organizer: Address) {
        organizer.require_auth();

        let event_key = DataKey::Event(event_id);
        let mut event = get_event_or_panic(&env, &event_key);

        require_organizer_or_admin(&env, &organizer, &event.organizer);

        event.active = false;
        env.storage().persistent().set(&event_key, &event);
    }
}

fn get_event_or_panic(env: &Env, event_key: &DataKey) -> Event {
    env.storage()
        .persistent()
        .get(event_key)
        .unwrap_or_else(|| panic_with_error(env, Error::EventNotFound))
}

fn get_ticket_or_panic(env: &Env, ticket_key: &DataKey) -> Ticket {
    env.storage()
        .persistent()
        .get(ticket_key)
        .unwrap_or_else(|| panic_with_error(env, Error::TicketNotFound))
}

fn require_organizer_or_admin(env: &Env, caller: &Address, organizer: &Address) {
    if caller == organizer {
        return;
    }

    let admin = env
        .storage()
        .persistent()
        .get::<_, Address>(&DataKey::Admin)
        .unwrap_or_else(|| panic_with_error(env, Error::NotInitialized));

    if caller != &admin {
        panic_with_error(env, Error::Unauthorized);
    }
}

fn panic_with_error(env: &Env, error: Error) -> ! {
    let _ = env;
    panic!("{:?}", error);
}

#[cfg(test)]
mod test;
