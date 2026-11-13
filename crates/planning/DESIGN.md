# mockspace-extra-planning

The planning layer of the registry and the rules a schema cannot state over it, held once for every client. It answers to `a_milestone_merge_lands_everything_finished_at_once_before_anything_new_starts` for what a milestone is, to `the_workflow_tools_are_one_data_view_the_reader_configures_with_presets` for the tools that list its rows, and to `a_question_is_a_ticket_filed_by_whoever_has_it_and_reaches_the_trunk_without_being_landed_by_hand` for a write reaching the trunk as part of the act that makes it. It depends on `mockspace-extra-ticket-model` and on nothing of mockspace, the tickets tool and the console build it beside that crate, and it touches no file. The tickets crate depends on it in no way: removing every planning file leaves a registry the tickets crate loads whole.

## The plan

`Plan::from_texts` takes each registry file as its path and its text, as the tickets crate does, and keeps every row of the planning namespaces it is given, wherever in the registry the file sits, so a member row beside its ticket and a milestone in a file of its own are read alike. A row is its namespace, its id and its `Fields`, the type the tickets crate already offers, so there is one reading of a row's fields and no second copy of it here. A row with no id, an id held twice in a namespace and a file that is not TOML are refused with a sentence naming the file. The namespaces read are named by the caller, each one a key the registry declares, so a planning namespace added is a declaration and a name in a list and not a change of this crate's types. The rows are kept in the order read.

The schema gate already holds each field to its type and each reference to a row that exists. This crate holds what the gate cannot: that the facts of a chain agree.

## Facts, chains and heads

A `member`, a `window`, a `milestone_scope` and a `plan_slot` are facts about one ticket, appended, each naming in `over` the row of the same ticket it was written over. `Plan::heads` gives the rows of one ticket in one namespace that no other row names in `over`: one head is the current fact, and the newest head by `at` is read for a fact that carries a stamp. More than one head is a fork, two writers over one predecessor, and it is a finding and not a silent loss.

## The rules

Each rule is a function of a `Plan` and gives its `Finding`s, the type of the tickets crate. `check_over` holds a row's `over` to a row of the same namespace about the same ticket or span, and the rows to no cycle. `check_forks` names every chain with more than one head. `check_stamp` holds `at` and `by` to both present or both absent, the absent pair being legacy. `check_spans` holds a span with `planned` true, an epoch, to a `within` naming a stretch and an `owner`, and no `after` or `until`; and a span with `planned` false, a stretch, to neither `within` nor `owner`. `check_plan` gives every finding the committed rows break, the same rules over every row, so a hand edit or a merge is held to what the makers are.

## Derived, not stored

`Plan::members_now` gives the span each ticket is in now and whether it is core there. A milestone is reached when a `milestone_merge` row names it, so `Plan::reached` asks and nothing stores it. A ticket's actual start is its first move into a state whose role row says it starts work, and where no such move exists the start is unknown and stays so.

| Type | What it is |
| --- | --- |
| `Plan` | Every planning row a registry holds, by namespace and id, in the order read. |
| `Row` | One planning row: its namespace, its id and its `Fields`. |
| `Finding` | The tickets crate's own, re-exported here: a row that breaks a rule, and the sentence saying how. |

## Names and days

`NAMESPACES` lists the planning namespaces a registry declares, each a key `mockspace.toml` holds, so the tickets tool and the console read one list and a namespace added is a name here. The `time` module reads a day: `day_of` takes a date, `YYYY-MM-DD`, or a stamp, `YYYY-MM-DDTHH:MM:SSZ`, and gives the days since 1970-01-01 it falls on, and nothing for any other text; `date_of` is its inverse. Days are the only unit the derived values are counted in.

## Derived values

`Facts::of` takes a `Plan`, the tickets crate's `Store`, a day handed in as today and a number of days as the stall threshold, and gives for each ticket the facts below. Nothing in it is stored or written back, and nothing in it reads a clock, so the same rows give the same facts. The roles come from the rows: a state's class from its own flags, whether a kind is work or a complication from its `plan_kind_role` row, whether a state starts work from its `plan_state_role` row, so no state and no kind is named in the code.

The class of a ticket is that of the state it stands in, `Class`: done when the state closes and satisfies, dropped when it closes and does not, active when it is held with its needs met, stopped when it is held with its needs not met, and open otherwise. A ticket that never moved stands in its kind's initial state. The actual start is the first move into a state whose role starts work and is unknown where there is none; the actual end is the first move into a closing state after it, or after the first move of all where no start is known, and is unknown where the ticket is not closed. An unknown stays unknown and is never replaced by a guess. A ticket is blocked when it is work, not closed, and needs a ticket no state of theirs satisfies. It is stalled when it is active and its newest move is more than the threshold days before today. Its window is the planned start and end of the first row of its chain, the one naming nothing in `over`, so the plan as first made is what a slip is counted against; its slip is its actual end, or today where it is still open, less the planned end, and is unknown where there is no window, no planned end, or it is neither ended nor past the planned end.

`SpanReport` is a span as the sitrep needs it: its members now, the core ones, the counts by class of the core work, its progress as the done over the done and open and active and stopped, the dropped being left out of both, its blocked, stalled and complication members (a member whose kind role is a complication), the earliest actual start and latest actual end of its members, its planned dates, and the late scope, the members that are scope of a milestone already reached and are not done. `MilestoneReport` is a milestone as the sitrep needs it: whether it is reached, the day of its first merge row, its planned day, its goal, its scope now by the newest `milestone_scope` row of each ticket, the counts by class and the progress over that scope. Both are given by `span_report` and `milestone_report`, and `Roadmap::of` gives the rows of the timeline: the stretches and the milestones with their days, the epochs in lanes by owner, and a bar for each ticket with its planned and its actual days.

## Facts written

A write is a row appended, never an edit. `Home` says where a row sits: in the file of a ticket, for a member, a window or a scope row, and in a planning file of its own for a span or a milestone. `member_fact`, `window_fact` and `scope_fact` make the row that replaces a ticket's current fact, naming the head of its chain in `over`, with the stamp and the writer. They refuse a chain that is a fork already, since the head is then not one row. `epoch_fact` and `milestone_fact` make a new epoch, which must name a stretch that exists and an owner, and a new milestone, whose goal and planned day are optional, and refuse an id that is held. Every row is checked by `check_plan` against the plan with it added, and a row that brings a finding is refused with the sentence, so a row the console writes is one the tickets tool would have written.

| Type | What it is |
| --- | --- |
| `Class` | What a ticket's state means to progress: done, dropped, active, stopped or open. `Class::from_flags` is the one place the four flags of a state, closed, held, needs met and satisfies, are turned into a class, `Class::of` calls it, and a reader that holds the flags and not a state, as the console does of a row, asks it and keeps no copy. |
| `Facts` | The derived facts of every ticket, by id, for one plan, store, day and threshold. |
| `TicketFacts` | One ticket's class, actual start and end, blocked, stalled, window and slip. |
| `SpanReport` | One span's members, counts by class, progress, blocked, stalled, complications, days and late scope. |
| `MilestoneReport` | One milestone's reached, days, goal, scope and progress. |
| `Roadmap` | The timeline: stretches, milestones, epochs in lanes by owner, and a bar for each ticket. |
| `Home` | Where an appended row sits: in a ticket's file, or in a planning file. |
| `NewRow` | A row to append: its namespace, id, home and fields. |
