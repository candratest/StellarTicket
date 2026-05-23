extern crate std;

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String, Symbol,
};

struct Fixture {
    env: Env,
    contract_id: Address,
    admin: Address,
    organizer: Address,
    user: Address,
}

impl Fixture {
    fn client(&self) -> StellarTicketClient {
        StellarTicketClient::new(&self.env, &self.contract_id)
    }
}

fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|ledger| {
        ledger.timestamp = 1_000;
    });

    let contract_id = env.register(StellarTicket, ());

    let admin = Address::generate(&env);
    let organizer = Address::generate(&env);
    let user = Address::generate(&env);

    Fixture {
        env,
        contract_id,
        admin,
        organizer,
        user,
    }
}

fn symbol(env: &Env, value: &str) -> Symbol {
    Symbol::new(env, value)
}

fn string(env: &Env, value: &str) -> String {
    String::from_str(env, value)
}

fn create_sample_event(env: &Env, client: &StellarTicketClient, organizer: &Address) -> Symbol {
    let event_id = symbol(env, "event_1");

    client.create_event(
        &event_id,
        organizer,
        &string(env, "Stellar Dev Night"),
        &string(env, "A community event for Soroban builders"),
        &string(env, "Jakarta"),
        &2_000,
        &100,
        &2,
    );

    event_id
}

#[test]
fn init_admin_succeeds() {
    let fixture = setup();
    let client = fixture.client();

    client.init_admin(&fixture.admin);
}

#[test]
fn create_event_succeeds() {
    let fixture = setup();
    let client = fixture.client();
    let event_id = create_sample_event(&fixture.env, &client, &fixture.organizer);

    let event = client.get_event(&event_id);

    assert_eq!(event.organizer, fixture.organizer);
    assert_eq!(event.sold_tickets, 0);
    assert_eq!(event.max_tickets, 2);
    assert!(event.active);
}

#[test]
fn user_buy_ticket_succeeds() {
    let fixture = setup();
    let client = fixture.client();
    let event_id = create_sample_event(&fixture.env, &client, &fixture.organizer);
    let ticket_id = symbol(&fixture.env, "ticket_1");

    client.buy_ticket(&event_id, &ticket_id, &fixture.user);

    let ticket = client.get_ticket(&ticket_id);
    let event = client.get_event(&event_id);

    assert_eq!(ticket.owner, fixture.user);
    assert_eq!(ticket.event_id, event_id);
    assert!(!ticket.used);
    assert!(ticket.valid);
    assert_eq!(ticket.issued_at, 1_000);
    assert_eq!(event.sold_tickets, 1);
}

#[test]
#[should_panic(expected = "ContractError(12)")]
fn user_cannot_buy_two_tickets_for_same_event() {
    let fixture = setup();
    let client = fixture.client();
    let event_id = create_sample_event(&fixture.env, &client, &fixture.organizer);

    client.buy_ticket(&event_id, &symbol(&fixture.env, "ticket_1"), &fixture.user);
    client.buy_ticket(&event_id, &symbol(&fixture.env, "ticket_2"), &fixture.user);
}

#[test]
#[should_panic(expected = "ContractError(11)")]
fn cannot_buy_ticket_when_sold_out() {
    let fixture = setup();
    let client = fixture.client();
    let event_id = create_sample_event(&fixture.env, &client, &fixture.organizer);
    let user_2 = Address::generate(&fixture.env);
    let user_3 = Address::generate(&fixture.env);

    client.buy_ticket(&event_id, &symbol(&fixture.env, "ticket_1"), &fixture.user);
    client.buy_ticket(&event_id, &symbol(&fixture.env, "ticket_2"), &user_2);
    client.buy_ticket(&event_id, &symbol(&fixture.env, "ticket_3"), &user_3);
}

#[test]
fn verify_ticket_true_before_used() {
    let fixture = setup();
    let client = fixture.client();
    let event_id = create_sample_event(&fixture.env, &client, &fixture.organizer);
    let ticket_id = symbol(&fixture.env, "ticket_1");

    client.buy_ticket(&event_id, &ticket_id, &fixture.user);

    assert!(client.verify_ticket(&ticket_id));
}

#[test]
fn use_ticket_marks_ticket_used() {
    let fixture = setup();
    let client = fixture.client();
    client.init_admin(&fixture.admin);
    let event_id = create_sample_event(&fixture.env, &client, &fixture.organizer);
    let ticket_id = symbol(&fixture.env, "ticket_1");

    client.buy_ticket(&event_id, &ticket_id, &fixture.user);
    client.use_ticket(&ticket_id, &fixture.organizer);

    let ticket = client.get_ticket(&ticket_id);
    assert!(ticket.used);
}

#[test]
fn verify_ticket_false_after_used() {
    let fixture = setup();
    let client = fixture.client();
    client.init_admin(&fixture.admin);
    let event_id = create_sample_event(&fixture.env, &client, &fixture.organizer);
    let ticket_id = symbol(&fixture.env, "ticket_1");

    client.buy_ticket(&event_id, &ticket_id, &fixture.user);
    client.use_ticket(&ticket_id, &fixture.organizer);

    assert!(!client.verify_ticket(&ticket_id));
}

#[test]
#[should_panic(expected = "ContractError(15)")]
fn non_organizer_cannot_use_ticket() {
    let fixture = setup();
    let client = fixture.client();
    client.init_admin(&fixture.admin);
    let event_id = create_sample_event(&fixture.env, &client, &fixture.organizer);
    let ticket_id = symbol(&fixture.env, "ticket_1");
    let stranger = Address::generate(&fixture.env);

    client.buy_ticket(&event_id, &ticket_id, &fixture.user);
    client.use_ticket(&ticket_id, &stranger);
}

#[test]
fn transfer_ticket_succeeds() {
    let fixture = setup();
    let client = fixture.client();
    let event_id = create_sample_event(&fixture.env, &client, &fixture.organizer);
    let ticket_id = symbol(&fixture.env, "ticket_1");
    let receiver = Address::generate(&fixture.env);

    client.buy_ticket(&event_id, &ticket_id, &fixture.user);
    client.transfer_ticket(&ticket_id, &fixture.user, &receiver);

    let ticket = client.get_ticket(&ticket_id);
    assert_eq!(ticket.owner, receiver);
}

#[test]
#[should_panic(expected = "ContractError(14)")]
fn used_ticket_cannot_be_transferred() {
    let fixture = setup();
    let client = fixture.client();
    client.init_admin(&fixture.admin);
    let event_id = create_sample_event(&fixture.env, &client, &fixture.organizer);
    let ticket_id = symbol(&fixture.env, "ticket_1");
    let receiver = Address::generate(&fixture.env);

    client.buy_ticket(&event_id, &ticket_id, &fixture.user);
    client.use_ticket(&ticket_id, &fixture.organizer);
    client.transfer_ticket(&ticket_id, &fixture.user, &receiver);
}

#[test]
fn cancel_event_makes_event_inactive() {
    let fixture = setup();
    let client = fixture.client();
    client.init_admin(&fixture.admin);
    let event_id = create_sample_event(&fixture.env, &client, &fixture.organizer);

    client.cancel_event(&event_id, &fixture.organizer);

    let event = client.get_event(&event_id);
    assert!(!event.active);
}
