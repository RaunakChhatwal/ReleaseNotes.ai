use leptos::*;

use crate::util::Ticket;

#[component]
pub fn TicketForm(
    ticket: ReadSignal<Ticket>,
    set_ticket: WriteSignal<Ticket>,
    tickets: ReadSignal<Vec<(usize, (ReadSignal<Ticket>, WriteSignal<Ticket>))>>,
    set_tickets: WriteSignal<Vec<(usize, (ReadSignal<Ticket>, WriteSignal<Ticket>))>>,
    id: usize
) -> impl IntoView {
    let delete_ticket = move |_| {
        set_tickets.update(|tickets| {
            tickets.retain(|&(ticket_id, (read_signal, write_signal))| {
                if ticket_id == id {
                    read_signal.dispose();
                    write_signal.dispose();
                    return false;
                } else {
                    return true;
                }
            });
        });
    };

    view! {
        <div class="relative p-4 text-[0.9rem] border-2 border-black">
            <button
                class="absolute top-0 right-0 m-2 px-[0.3em] py-[0.15em] border-2 border-black hover:bg-gray-200"
                style:display=move || (tickets().len() <= 1).then(|| "None")
                on:click=delete_ticket
            >"Remove"</button>
            <div class="grid grid-cols-full gap-4 p-[0.75vw]">
                <div>
                    <p class="text-[0.95rem]">"Summary"</p>
                    <input
                        class="w-full px-[3px] bg-gray-200 border-2 border-black"
                        type = "text"
                        value=move || ticket().summary
                        on:input = move |event| set_ticket.update(|ticket|
                            ticket.summary = event_target_value(&event)
                        ) />
                </div>
                <div>
                    <p class="text-[0.95rem]">"Description"</p>
                    <textarea
                        class="w-full h-[10rem] px-[3px] bg-gray-200 border-2 border-black"
                        type = "text"
                        defaultValue=move || ticket().description
                        on:input = move |event| set_ticket.update(|ticket|
                            ticket.description = event_target_value(&event))
                    >{ticket.get_untracked().description}</textarea>
                </div>
            </div>
        </div>
    }
}