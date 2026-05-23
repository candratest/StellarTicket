# StellarTicket

StellarTicket is a Soroban smart contract for digital event tickets on Stellar.
Organizers can create events, users can buy or claim tickets, tickets can be
verified, used at event check-in, transferred to another user, and events can be
canceled. All contract data is stored in Soroban persistent storage.

This MVP intentionally uses dummy payments: `buy_ticket` does not transfer any
token yet.

## Contract

The contract is implemented in the `contracts/notes` crate. The Rust contract
type is named `StellarTicket`, and the crate follows the template package name
`notes`. It uses `soroban-sdk` version `25` from the workspace dependency.

## Data Model

`Event`

- `organizer: Address`
- `name: String`
- `description: String`
- `location: String`
- `event_date: u64`
- `ticket_price: i128`
- `max_tickets: u32`
- `sold_tickets: u32`
- `active: bool`

`Ticket`

- `ticket_id: Symbol`
- `event_id: Symbol`
- `owner: Address`
- `issued_at: u64`
- `used: bool`
- `valid: bool`

## Storage Keys

- `Admin`
- `Event(Symbol)`
- `Ticket(Symbol)`
- `UserTicket(Address, Symbol)`

## Public Functions

- `init_admin(admin)` initializes the admin once.
- `create_event(...)` creates an active event owned by the organizer.
- `buy_ticket(event_id, ticket_id, buyer)` issues one valid ticket to the buyer.
- `get_event(event_id)` returns an event.
- `get_ticket(ticket_id)` returns a ticket.
- `verify_ticket(ticket_id)` returns whether a ticket exists, is valid, unused,
  and belongs to an active event.
- `use_ticket(ticket_id, checker)` marks a ticket as used. The checker must be
  the event organizer or admin.
- `transfer_ticket(ticket_id, from, to)` transfers an unused valid ticket to
  another user.
- `cancel_event(event_id, organizer)` marks an event inactive. The caller must be
  the event organizer or admin.

## Tests

Run the unit tests from the workspace root:

```bash
cargo test
```

The tests cover admin initialization, event creation, ticket purchase rules,
sold-out events, verification before and after use, check-in authorization,
ticket transfers, transfer restrictions for used tickets, and event cancelation.
