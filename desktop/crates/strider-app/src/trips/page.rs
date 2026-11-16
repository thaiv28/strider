use super::*;
use strider_core::units::Grams;
impl AppView {
    pub(in crate::app) fn render_trips(
        &mut self,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = Theme::get(cx);
        let body = if let Some(d) = self.trips.detail.clone() {
            self.render_trip_page(&d, w, cx)
        } else if self.area == "Basecamp" {
            self.render_basecamp(cx)
        } else {
            self.render_trip_list(cx)
        };
        strider_ui::page_body(
            [div()
                .id("trips-scroll")
                .overflow_y_scroll()
                .flex_1()
                .min_h_0()
                .child(body)
                .into_any_element()],
            t,
        )
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .into_any_element()
    }
    fn trip_row(&self, trip: &TripSummary, cx: &Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        let id = trip.sync_id.clone();
        let delete = id.clone();
        let route = trip.route;
        div()
            .id(SharedString::from(id.clone()))
            .min_h(px(64.))
            .flex()
            .items_center()
            .gap_3()
            .px_4()
            .border_b_1()
            .border_color(t.line)
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| this.open_trip(id.clone(), cx)))
            .child(strider_ui::route_thumbnail(
                trip.track.clone().unwrap_or_default(),
                t,
            ))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(trip.name.clone())
                    .child(div().meta(t).child(trip.region.clone().unwrap_or_default())),
            )
            .child(
                div()
                    .w(px(100.))
                    .meta(t)
                    .child(trip.start_date.clone().unwrap_or_else(|| "No dates".into())),
            )
            .child(div().w(px(75.)).meta(t).child(trip.status.clone()))
            .child(
                div().w(px(60.)).readout().child(
                    trip.nights
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "—".into()),
                ),
            )
            .child(
                div().w(px(60.)).readout().child(
                    route
                        .distance_mi
                        .map(|n| format!("{n:.1}"))
                        .unwrap_or_else(|| "—".into()),
                ),
            )
            .child(
                div().w(px(70.)).readout().child(
                    route
                        .elevation_ft
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "—".into()),
                ),
            )
            .child(
                div()
                    .w(px(80.))
                    .readout()
                    .child(fmt_weight(Some(Grams(trip.base_g as i32)))),
            )
            .child(
                div()
                    .w(px(80.))
                    .readout()
                    .child(fmt_weight(Some(Grams(trip.skin_out_g as i32)))),
            )
            .when(trip.access == "owner", |d| {
                d.child(
                    strider_ui::button(
                        eid("delete-trip", &delete.clone()),
                        "Delete",
                        ButtonVariant::Danger,
                        t,
                    )
                    .debug_selector(|| "trip-delete".to_owned())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.command(
                            Command::DeleteTrip {
                                trip: delete.clone(),
                            },
                            cx,
                        );
                    })),
                )
            })
            .into_any_element()
    }
    fn shared_card(&self, trip: &TripSummary, cx: &Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        let id = trip.sync_id.clone();
        let edit = trip.access == "edit";
        let when = strider_core::format::fmt_date_range(trip.start_date.as_deref(), trip.nights)
            .unwrap_or_else(|| "No dates".into());
        let detail = [
            trip.region.clone(),
            Some(when),
            trip.route.distance_mi.map(|d| format!("{d} mi")),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");
        strider_ui::card(t)
            .id(eid("shared", &trip.sync_id))
            .debug_selector(|| "shared-card".to_owned())
            .w(px(340.))
            .p_4()
            .flex()
            .flex_col()
            .gap_2()
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| this.open_trip(id.clone(), cx)))
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap_3()
                    .child(div().card_heading().child(trip.name.clone()))
                    .child(strider_ui::badge(
                        if edit { "Can edit" } else { "View only" },
                        if edit { "current" } else { "neutral" },
                        t,
                    )),
            )
            .child(div().meta(t).readout().child(detail))
            .into_any_element()
    }
    /// The Logbook: your trips by status, or the trips shared with you (port of `logbook-view.tsx`).
    fn render_trip_list(&self, cx: &Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        let filter = self.trips.filter;
        let shared: Vec<TripSummary> = self
            .all_trips()
            .into_iter()
            .filter(|t| t.access != "owner")
            .collect();
        let chips = div().flex().flex_wrap().gap_2().children(
            [
                ("all", "All".to_owned()),
                ("planned", "Planned".into()),
                ("idea", "Ideas".into()),
                ("completed", "Completed".into()),
                ("shared", format!("Shared with me ({})", shared.len())),
            ]
            .map(|(value, label)| {
                strider_ui::chip(value, label, filter == value, t).on_click(cx.listener(
                    move |this, _, _, cx| {
                        this.trips.filter = value;
                        cx.notify();
                    },
                ))
            }),
        );
        let mut body = div().flex().flex_col().gap_5().child(chips);
        if filter == "shared" {
            if shared.is_empty() {
                body = body.child(
                    div()
                        .meta(t)
                        .child("Open a trip link while signed in to find it here."),
                );
            }
            body = body.child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_3()
                    .children(shared.iter().map(|trip| self.shared_card(trip, cx))),
            );
            return body.into_any_element();
        }
        let mut shown = 0;
        for (label, trips) in [
            ("In progress", &self.trips.list.in_progress),
            ("Upcoming", &self.trips.list.upcoming),
            ("Past", &self.trips.list.past),
        ] {
            let trips: Vec<&TripSummary> = trips
                .iter()
                .filter(|t| t.access == "owner" && (filter == "all" || t.status == filter))
                .collect();
            if trips.is_empty() {
                continue;
            }
            shown += trips.len();
            body = body.child(
                div()
                    .child(div().section_heading().mb_3().child(label))
                    .child(
                        strider_ui::card(t)
                            .child(div().px_4().py_3().meta(t).child(
                                "Trip                                      Date          Status      Nights    Miles    Gain ft     Base     Skin-out",
                            ))
                            .children(trips.into_iter().map(|trip| self.trip_row(trip, cx))),
                    ),
            );
        }
        if shown == 0 {
            body = body.child(div().meta(t).child(if filter == "all" {
                "No trips yet. Create a trip or import a trip JSON file."
            } else {
                "No trips in this section yet."
            }));
        }
        body.child(
            strider_ui::button("undo-trip", "Undo", ButtonVariant::Ghost, t)
                .on_click(cx.listener(|this, _, _, cx| this.undo(false, cx))),
        )
        .into_any_element()
    }
    fn render_basecamp(&self, cx: &Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        let mut all = self.all_trips();
        let next = all
            .iter()
            .filter(|t| t.status == "planned")
            .min_by_key(|t| t.start_date.as_deref().unwrap_or("9999"));
        let mut body = div().flex().flex_col().gap_5();
        if let Some(next) = next {
            let id = next.sync_id.clone();
            body = body.child(
                strider_ui::card(t)
                    .p_5()
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| this.open_trip(id.clone(), cx)),
                    )
                    .child(div().text_color(t.signal).child("Next up"))
                    .child(div().section_heading().child(next.name.clone()))
                    .child(div().meta(t).child(format!(
                        "{} · {:.1} mi · {} ft",
                        next.start_date.as_deref().unwrap_or("No dates"),
                        next.route.distance_mi.unwrap_or(0.),
                        next.route.elevation_ft.unwrap_or(0)
                    )))
                    .child(strider_ui::route_thumbnail(
                        next.track.clone().unwrap_or_default(),
                        t,
                    )),
            );
        }
        all.sort_by(|a, b| b.start_date.cmp(&a.start_date));
        body = body
            .child(div().section_heading().child("Recent trips"))
            .children(
                all.iter()
                    .take(6)
                    .map(|trip| strider_ui::card(t).child(self.trip_row(trip, cx))),
            );
        if all.is_empty() {
            body = body.child(div().meta(t).child("No trips yet. Create your first trip."));
        }
        body.child(
            div()
                .flex()
                .gap_2()
                .children(["Gear", "Food", "Trips"].into_iter().map(|area| {
                    strider_ui::button(area, area, ButtonVariant::Ghost, t).on_click(cx.listener(
                        move |this, _, w, cx| {
                            this.request_navigation(Navigation::Area(area), w, cx)
                        },
                    ))
                })),
        )
        .into_any_element()
    }
    fn trip_button(
        &self,
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        cmd: Command,
        cx: &Context<Self>,
    ) -> AnyElement {
        strider_ui::button(id, label, ButtonVariant::Ghost, Theme::get(cx))
            .on_click(cx.listener(move |this, _, _, cx| this.command(cmd.clone(), cx)))
            .into_any_element()
    }
    fn render_trip_page(&self, d: &TripDetail, w: &Window, cx: &Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        let editable = d.access != "view";
        let trip = text(&d.fields, "sync_id");
        let mut route = strider_ui::card(t).flex().flex_col();
        let mut options = vec![("Attach GPX / permit", "attach"), ("Export GPX", "export")];
        if !d.points.is_empty() {
            options.push(("Download map for offline", "download"));
        }
        if editable {
            options.extend([
                ("Reverse route", "reverse"),
                ("Remove route", "remove"),
                ("Clear campsites", "clear"),
            ]);
        }
        route = route.child(
            div()
                .flex()
                .justify_between()
                .child(
                    strider_ui::button(
                        "toggle-map",
                        if self.trips.minimized {
                            "Show map and elevation"
                        } else {
                            "Hide map"
                        },
                        ButtonVariant::Ghost,
                        t,
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.trips.minimized = !this.trips.minimized;
                        cx.notify();
                    })),
                )
                .child(
                    self.choice(
                        "route-options",
                        "Route options",
                        options
                            .into_iter()
                            .filter(|(_, key)| editable || matches!(*key, "export" | "download"))
                            .map(|(l, k)| (k.into(), l.into()))
                            .collect(),
                        cx,
                        |this, key, _, cx| {
                            let Some(trip) = this.trips.selected.clone() else {
                                return;
                            };
                            match key.as_str() {
                                "attach" => this.pick_trip_file(cx),
                                "export" => this.export_trip_gpx(cx),
                                "download" => this.open_map_sheet(cx),
                                "reverse" => this.command(Command::ReverseGpx { trip }, cx),
                                "remove" => this.command(Command::RemoveGpx { trip }, cx),
                                "clear" => this.command(Command::ClearCampsites { trip }, cx),
                                _ => {}
                            }
                        },
                    ),
                ),
        );
        if !self.trips.minimized {
            route = route.child(self.trips.map.clone());
            if let Some(message) = self.map_message(d) {
                route = route.child(div().px_4().py_2().meta(t).child(message));
            }
            route = route.child(self.trips.profile.clone());
        }
        if d.points.is_empty() {
            route = route.child(
                div()
                    .p_4()
                    .meta(t)
                    .child("Upload or drop a GPX file to see the route and elevation profile."),
            );
        }
        let sections = div().flex().flex_wrap().gap_1p5().children(
            ["Logistics", "Planning", "Gear", "Food", "Weather", "Report"]
                .into_iter()
                .map(|section| {
                    strider_ui::chip(section, section, self.trips.section == section, t).on_click(
                        cx.listener(move |this, _, _, cx| {
                            this.trips.section = section;
                            this.load_panels();
                            cx.notify();
                        }),
                    )
                }),
        );
        let content = match self.trips.section {
            "Gear" => self.render_trip_gear(d, cx),
            "Food" => self.render_trip_food(d, cx),
            "Planning" => self.render_trip_planning(d, cx),
            "Weather" => self.render_trip_weather(cx),
            "Report" => self.render_trip_report(d, cx),
            _ => self.render_trip_logistics(d, cx),
        };
        let _ = trip;
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .gap_3()
                    .when(w.viewport_size().width >= px(1100.), |v| v.flex_row())
                    .when(w.viewport_size().width < px(1100.), |v| v.flex_col())
                    .child(div().flex_1().min_w_0().child(route))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(sections)
                            .child(content),
                    ),
            )
            .when(editable, |v| {
                v.child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            strider_ui::button("trip-undo", "Undo", ButtonVariant::Ghost, t)
                                .on_click(cx.listener(|this, _, _, cx| this.undo(false, cx))),
                        )
                        .child(
                            strider_ui::button("trip-redo", "Redo", ButtonVariant::Ghost, t)
                                .on_click(cx.listener(|this, _, _, cx| this.undo(true, cx))),
                        ),
                )
            })
            .into_any_element()
    }
    fn note(
        &self,
        d: &TripDetail,
        title: &'static str,
        key: &'static str,
        cx: &Context<Self>,
    ) -> AnyElement {
        let t = Theme::get(cx);
        strider_ui::card(t)
            .debug_selector(|| format!("note-{key}"))
            .p_4()
            .child(div().card_heading().child(title))
            .child(div().py_3().child(text(&d.fields, key)))
            .when(d.access != "view", |v| {
                v.child(
                    strider_ui::button(key, "Edit", ButtonVariant::Ghost, t).on_click(cx.listener(
                        move |this, _, w, cx| this.edit_trip(EditKind::Note(key.into()), w, cx),
                    )),
                )
            })
            .into_any_element()
    }
    fn render_trip_logistics(&self, d: &TripDetail, cx: &Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        let trip = text(&d.fields, "sync_id");
        let editable = d.access != "view";
        let mut details = strider_ui::card(t)
            .p_4()
            .child(div().card_heading().child("Details"))
            .child(div().py_3().child(format!(
                "{} · {} nights · {} people",
                text(&d.fields, "start_date"),
                text(&d.fields, "nights"),
                text(&d.fields, "party_size")
            )))
            .child(div().child(format!("Trailhead: {}", text(&d.fields, "trailhead"))));
        if editable {
            details = details.child(
                self.choice(
                    "trip-status",
                    text(&d.fields, "status"),
                    ["idea", "planned", "completed"]
                        .into_iter()
                        .map(|s| (s.into(), s.into()))
                        .collect(),
                    cx,
                    |this, s, _, cx| {
                        if let Some(trip) = this.trips.selected.clone() {
                            this.command(
                                Command::UpdateTrip {
                                    trip,
                                    patch: Row::from([("status".into(), Value::Text(s))]),
                                },
                                cx,
                            );
                        }
                    },
                ),
            );
        }
        let mut permit = strider_ui::card(t)
            .p_4()
            .child(div().card_heading().child("Permit document"));
        if let Some(p) = &d.permit {
            let missing = p.state == "missing";
            permit = permit
                .child(
                    div()
                        .meta(t)
                        .child(format!("{} · {} KB", p.filename, p.size_bytes / 1024)),
                )
                .when(missing, |v| {
                    v.child(div().meta(t).child(if self.account.syncing() {
                        "Downloading…"
                    } else {
                        "Not downloaded"
                    }))
                })
                .when(!missing, |v| {
                    v.child(
                        strider_ui::button("open-permit", "Open permit", ButtonVariant::Ghost, t)
                            .on_click(cx.listener(|this, _, _, cx| this.open_permit(cx))),
                    )
                });
            if editable {
                permit = permit.child(self.trip_button(
                    "delete-permit",
                    "Delete permit",
                    Command::DeletePermit { trip: trip.clone() },
                    cx,
                ));
            }
        }
        if editable {
            permit = permit.child(
                strider_ui::button(
                    "pick-permit",
                    "Choose PDF or image",
                    ButtonVariant::Ghost,
                    t,
                )
                .on_click(cx.listener(|this, _, _, cx| this.pick_trip_file(cx))),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(details)
            .child(permit)
            .child(self.note(d, "Permit notes", "permit_notes", cx))
            .child(self.note(d, "Driving notes", "driving_notes", cx))
            .into_any_element()
    }
    fn render_trip_planning(&self, d: &TripDetail, cx: &Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        let trip = text(&d.fields, "sync_id");
        let edit = d.access != "view";
        let mut days = strider_ui::card(t)
            .p_4()
            .child(div().card_heading().child("Days"));
        for r in &d.days {
            let day = num(r, "day_number") as i32;
            days = days.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(format!(
                        "Day {day} · {} mi · {} ft",
                        text(r, "distance_mi"),
                        text(r, "elevation_gain_ft")
                    ))
                    .when(edit, |v| {
                        v.child(
                            strider_ui::button(
                                ("day-edit", day as usize),
                                "Edit",
                                ButtonVariant::Ghost,
                                t,
                            )
                            .on_click(cx.listener(
                                move |this, _, w, cx| this.edit_trip(EditKind::Day(day), w, cx),
                            )),
                        )
                        .child(self.trip_button(
                            ("day-delete", day as usize),
                            "Remove",
                            Command::DeleteTripDay {
                                trip: trip.clone(),
                                id: text(r, "sync_id"),
                            },
                            cx,
                        ))
                    }),
            );
        }
        if edit {
            days = days.child(self.trip_button(
                "add-day",
                "Add day",
                Command::AddTripDay { trip: trip.clone() },
                cx,
            ));
        }
        let mut sites = strider_ui::card(t)
            .p_4()
            .child(div().card_heading().child("Campsites"));
        for r in &d.campsites {
            sites = sites.child(
                div()
                    .flex()
                    .items_center()
                    .child(format!(
                        "Night {} · {:.1} mi",
                        text(r, "night"),
                        num(r, "distance_mi")
                    ))
                    .when(edit, |v| {
                        v.child(self.trip_button(
                            eid("remove-camp", &text(r, "sync_id")),
                            "Remove",
                            Command::RemoveCampsite {
                                trip: trip.clone(),
                                id: text(r, "sync_id"),
                            },
                            cx,
                        ))
                    }),
            );
        }
        if d.campsites.is_empty() {
            sites = sites.child(
                div()
                    .meta(t)
                    .child("Click the map or profile to add a campsite."),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(days)
            .child(sites)
            .child(self.note(d, "Water sources", "water_sources", cx))
            .child(self.note(d, "Planning notes", "planning_notes", cx))
            .into_any_element()
    }
    fn render_trip_gear(&self, d: &TripDetail, cx: &Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        let trip = text(&d.fields, "sync_id");
        let edit = d.access != "view";
        let mut body = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(div().readout().child(format!(
                "Base {} · Worn {} · {}/{} packed",
                fmt_weight(Some(Grams(d.weights.base as i32))),
                fmt_weight(Some(Grams(d.weights.worn as i32))),
                d.packed_count,
                d.gear.len()
            )));
        if edit {
            body = body
                .child(
                    self.choice(
                        "trip-add-gear",
                        "Add gear",
                        self.all_gear
                            .iter()
                            .map(|g| (g.id.clone(), g.name.clone()))
                            .collect(),
                        cx,
                        |this, gear, _, cx| {
                            if let Some(trip) = this.trips.selected.clone() {
                                this.command(Command::AddTripGear { trip, gear }, cx);
                            }
                        },
                    ),
                )
                .child(
                    self.choice(
                        "trip-seed",
                        "Seed from loadout",
                        self.loadouts
                            .loadouts
                            .iter()
                            .map(|l| (l.id.clone(), l.name.clone()))
                            .collect(),
                        cx,
                        |this, loadout, _, cx| {
                            if let Some(trip) = this.trips.selected.clone() {
                                this.command(Command::SeedTripFromLoadout { trip, loadout }, cx);
                            }
                        },
                    ),
                );
        }
        for (category, rows) in &d.groups {
            let mut group = strider_ui::card(t)
                .p_4()
                .child(div().card_heading().child(category.clone()));
            for (i, r) in rows.iter().enumerate() {
                let id = text(r, "sync_id");
                let qty_id = id.clone();
                let class = if text(r, "weight_class") == "worn" {
                    "base"
                } else {
                    "worn"
                };
                let mut row = div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .items_center()
                    .child(div().flex_1().child(text(r, "snapshot_name")))
                    .child(div().readout().child(format!(
                        "{} × {}",
                        text(r, "quantity"),
                        fmt_weight(Some(Grams(num(r, "snapshot_weight_g") as i32)))
                    )));
                if edit {
                    row = row
                        .child(self.trip_button(
                            eid("packed", &id.clone()),
                            if num(r, "packed") != 0. {
                                "Packed"
                            } else {
                                "Pack"
                            },
                            Command::SetTripGearPacked {
                                trip: trip.clone(),
                                id: id.clone(),
                                packed: num(r, "packed") == 0.,
                            },
                            cx,
                        ))
                        .child(self.trip_button(
                            eid("class", &id.clone()),
                            class,
                            Command::SetTripGearClass {
                                trip: trip.clone(),
                                id: id.clone(),
                                class: class.into(),
                            },
                            cx,
                        ))
                        .child(
                            strider_ui::button(
                                eid("qty", &id.clone()),
                                "Qty",
                                ButtonVariant::Ghost,
                                t,
                            )
                            .on_click(cx.listener(
                                move |this, _, w, cx| {
                                    this.edit_trip(EditKind::Gear(qty_id.clone()), w, cx)
                                },
                            )),
                        )
                        .child(self.trip_button(
                            eid("remove-gear", &id.clone()),
                            "Remove",
                            Command::RemoveTripGear {
                                trip: trip.clone(),
                                id: id.clone(),
                            },
                            cx,
                        ));
                    let mut order: Vec<_> = rows.iter().map(|r| text(r, "sync_id")).collect();
                    if i > 0 {
                        order.swap(i - 1, i);
                    }
                    row = row.child(self.trip_button(
                        eid("move-up", &id.clone()),
                        "Move up",
                        Command::MoveTripGear {
                            trip: trip.clone(),
                            id: id.clone(),
                            category: category.clone(),
                            ordered_ids: order,
                        },
                        cx,
                    ));
                    let item = id.clone();
                    row = row.child(
                        self.choice(
                            eid("move-category", &id),
                            "Move",
                            self.categories
                                .iter()
                                .map(|c| (c.name.clone(), c.name.clone()))
                                .collect(),
                            cx,
                            move |this, category, _, cx| {
                                if let Some(d) = &this.trips.detail {
                                    let mut order: Vec<_> = d
                                        .gear
                                        .iter()
                                        .filter(|g| {
                                            text(g, "snapshot_category") == category
                                                && text(g, "sync_id") != item
                                        })
                                        .map(|g| text(g, "sync_id"))
                                        .collect();
                                    order.push(item.clone());
                                    this.command(
                                        Command::MoveTripGear {
                                            trip: text(&d.fields, "sync_id"),
                                            id: item.clone(),
                                            category,
                                            ordered_ids: order,
                                        },
                                        cx,
                                    );
                                }
                            },
                        ),
                    );
                }
                group = group.child(row);
            }
            body = body.child(group);
        }
        body.child(
            strider_ui::card(t)
                .p_4()
                .child(div().card_heading().child("Packing list"))
                .child(d.packing_list.clone().unwrap_or_default())
                .when(edit, |v| {
                    v.child(
                        strider_ui::button(
                            "edit-packing",
                            "Edit packing list",
                            ButtonVariant::Ghost,
                            t,
                        )
                        .on_click(
                            cx.listener(|this, _, w, cx| this.edit_trip(EditKind::Packing, w, cx)),
                        ),
                    )
                }),
        )
        .into_any_element()
    }
    fn render_trip_food(&self, d: &TripDetail, cx: &Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        let edit = d.access != "view";
        let trip = text(&d.fields, "sync_id");
        let mut body = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(div().readout().child(format!(
                "Trip food · {} kcal · {}",
                d.food.total_kcal,
                fmt_weight(Some(Grams(d.food.total_g as i32)))
            )));
        for day in &d.food.days {
            let number = day.day_number;
            let mut card = strider_ui::card(t)
                .p_4()
                .child(div().card_heading().child(format!(
                    "Day {number} · {} / {} kcal",
                    day.planned_kcal, day.target_kcal
                )));
            for e in &day.entries {
                let id = e.id.clone();
                card = card.child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap_1()
                        .child(div().flex_1().child(format!(
                            "{} · {} kcal · {}",
                            e.name,
                            e.kcal,
                            fmt_weight(Some(Grams(e.weight_g as i32)))
                        )))
                        .when(edit, |v| {
                            v.child(
                                strider_ui::button(
                                    eid("meal-qty", &id.clone()),
                                    "Qty",
                                    ButtonVariant::Ghost,
                                    t,
                                )
                                .on_click(cx.listener(
                                    move |this, _, w, cx| {
                                        this.edit_trip(EditKind::Meal(id.clone()), w, cx)
                                    },
                                )),
                            )
                            .child(self.trip_button(
                                eid("meal-copy", &e.id.clone()),
                                "All days",
                                Command::CopyTripMealToAll {
                                    trip: trip.clone(),
                                    id: e.id.clone(),
                                },
                                cx,
                            ))
                            .child(self.trip_button(
                                eid("meal-remove", &e.id.clone()),
                                "Remove",
                                Command::RemoveTripMeal {
                                    trip: trip.clone(),
                                    id: e.id.clone(),
                                },
                                cx,
                            ))
                        }),
                );
            }
            if edit {
                card = card
                    .child(
                        self.choice(
                            ("add-trip-meal", number as usize),
                            "Add meal",
                            self.food
                                .meals
                                .iter()
                                .map(|m| (m.id.clone(), m.name.clone()))
                                .collect(),
                            cx,
                            move |this, meal, _, cx| {
                                if let Some(trip) = this.trips.selected.clone() {
                                    this.command(
                                        Command::AddTripMeal {
                                            trip,
                                            day: number,
                                            entry: AddTripFood::Meal { meal, servings: 1. },
                                        },
                                        cx,
                                    );
                                }
                            },
                        ),
                    )
                    .child(
                        self.choice(
                            ("add-trip-ingredient", number as usize),
                            "Add ingredient",
                            self.food
                                .ingredients
                                .iter()
                                .map(|i| (i.id.clone(), i.input.name.clone()))
                                .collect(),
                            cx,
                            move |this, ingredient, _, cx| {
                                if let Some(trip) = this.trips.selected.clone() {
                                    this.command(
                                        Command::AddTripMeal {
                                            trip,
                                            day: number,
                                            entry: AddTripFood::Ingredient {
                                                ingredient,
                                                grams: 50.,
                                            },
                                        },
                                        cx,
                                    );
                                }
                            },
                        ),
                    );
            }
            body = body.child(card);
        }
        let people = num(&d.fields, "grocery_people")
            .max(num(&d.fields, "party_size"))
            .max(1.);
        let shopping = strider_ui::card(t)
            .p_4()
            .child(
                div()
                    .card_heading()
                    .child(format!("Shopping list · {people:.0} people")),
            )
            .children(d.shopping.iter().map(|i| {
                div().py_2().readout().child(format!(
                    "{} · {:.0} g",
                    i.name,
                    i.grams as f64 * people
                ))
            }));
        body.child(shopping).into_any_element()
    }
    fn render_trip_report(&self, d: &TripDetail, cx: &Context<Self>) -> AnyElement {
        let t = Theme::get(cx);
        let body = div().flex().flex_col().gap_3().child(
            strider_ui::card(t)
                .p_4()
                .child(div().card_heading().child("Trip report"))
                .child(format!(
                    "{}\n{:.1} mi · {} ft gain\nPack {} · {} kcal planned",
                    text(&d.fields, "name"),
                    d.route.distance_mi.unwrap_or(0.),
                    d.route.elevation_ft.unwrap_or(0),
                    fmt_weight(Some(Grams(d.weights.pack as i32))),
                    d.food.total_kcal
                )),
        );
        if d.access == "owner" {
            body.child(self.note(d, "Trip report (private notes)", "trip_report", cx))
                .into_any_element()
        } else {
            body.into_any_element()
        }
    }
}
