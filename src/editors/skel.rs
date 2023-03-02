use std::path::Path;

use crate::{
    app::{Icons, SkelEditorState, SkelMode},
    path::folder_editor_title,
    widgets::enum_combo_box,
    EditorResponse,
};
use egui::{special_emojis::GITHUB, Button, CollapsingHeader, Label, RichText, ScrollArea};
use egui_dnd::DragDropItem;
use log::error;
use rfd::FileDialog;
use ssbh_data::{prelude::*, skel_data::BoneData};

struct SkelBoneIndex(usize);

impl DragDropItem for SkelBoneIndex {
    fn id(&self) -> egui::Id {
        egui::Id::new("bone").with(self.0)
    }
}

pub fn skel_editor(
    ctx: &egui::Context,
    folder_name: &Path,
    file_name: &str,
    skel: &mut SkelData,
    state: &mut SkelEditorState,
    icons: &Icons,
    dark_mode: bool,
) -> EditorResponse {
    let mut open = true;
    let mut changed = false;
    let mut saved = false;

    let title = folder_editor_title(folder_name, file_name);
    egui::Window::new(format!("Skel Editor ({title})"))
        .resizable(true)
        .open(&mut open)
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Save").clicked() {
                        ui.close_menu();

                        let file = Path::new(folder_name).join(file_name);
                        if let Err(e) = skel.write_to_file(&file) {
                            error!("Failed to save {:?}: {}", file, e);
                        } else {
                            saved = true;
                        }
                    }

                    if ui.button("Save As...").clicked() {
                        ui.close_menu();

                        if let Some(file) = FileDialog::new()
                            .add_filter("Skel", &["nusktb"])
                            .save_file()
                        {
                            if let Err(e) = skel.write_to_file(&file) {
                                error!("Failed to save {:?}: {}", file, e);
                            }
                        }
                    }
                });

                ui.menu_button("Skeleton", |ui| {
                    if ui
                        .add(Button::new("Match Reference Bone Order...").wrap(false))
                        .clicked()
                    {
                        ui.close_menu();

                        if let Some(file) = FileDialog::new()
                            .add_filter("Skel", &["nusktb"])
                            .pick_file()
                        {
                            match SkelData::from_file(&file) {
                                Ok(reference) => match_skel_order(skel, &reference),
                                Err(e) => error!("Failed to read {:?}: {}", file, e),
                            }
                        }
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button(format!("{GITHUB} Skel Editor Wiki")).clicked() {
                        ui.close_menu();

                        let link = "https://github.com/ScanMountGoat/ssbh_editor/wiki/Skel-Editor";
                        if let Err(e) = open::that(link) {
                            log::error!("Failed to open {link}: {e}");
                        }
                    }
                });
            });
            ui.separator();

            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut state.mode,
                    SkelMode::List,
                    RichText::new("List").heading(),
                );
                ui.selectable_value(
                    &mut state.mode,
                    SkelMode::Hierarchy,
                    RichText::new("Hierarchy").heading(),
                );
            });
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| match state.mode {
                    SkelMode::List => {
                        changed |= edit_bones_list(ui, skel, state, icons, dark_mode);
                    }
                    SkelMode::Hierarchy => {
                        changed |= edit_bones_hierarchy(ui, skel);
                    }
                });
        });

    EditorResponse {
        open,
        changed,
        saved,
    }
}

fn edit_bones_list(
    ui: &mut egui::Ui,
    skel: &mut SkelData,
    state: &mut SkelEditorState,
    icons: &Icons,
    dark_mode: bool,
) -> bool {
    let mut changed = false;

    // TODO: Do this without clone?
    let other_bones = skel.bones.clone();

    // TODO: Avoid allocating here.
    let mut items: Vec<_> = (0..skel.bones.len())
        .into_iter()
        .map(SkelBoneIndex)
        .collect();

    let response = state.dnd.ui(ui, items.iter_mut(), |item, ui, handle| {
        ui.horizontal(|ui| {
            let bone = &mut skel.bones[item.0];

            handle.ui(ui, item, |ui| {
                ui.add(icons.draggable(ui, dark_mode));
            });

            // Grids don't work with egui_dnd, so set the label size manually.
            // Use a workaround for left aligning the text.
            // TODO: Highlight the selected bone on hover.
            let (_, rect) = ui.allocate_space(egui::Vec2::new(250.0, 20.0));
            ui.child_ui(rect, egui::Layout::left_to_right(egui::Align::Center))
                .add(Label::new(&bone.name).sense(egui::Sense::click()));

            let id = egui::Id::new("bone").with(item.0);
            let parent_bone_name = bone
                .parent_index
                .and_then(|i| other_bones.get(i))
                .map(|p| p.name.as_str())
                .unwrap_or("None");

            ui.label("Parent Bone");
            egui::ComboBox::from_id_source(id)
                .selected_text(parent_bone_name)
                .width(250.0)
                .show_ui(ui, |ui| {
                    changed |= ui
                        .selectable_value(&mut bone.parent_index, None, "None")
                        .changed();
                    ui.separator();
                    // TODO: Is there a way to make this not O(N^2)?
                    for (other_i, other_bone) in other_bones.iter().enumerate() {
                        if item.0 != other_i {
                            changed |= ui
                                .selectable_value(
                                    &mut bone.parent_index,
                                    Some(other_i),
                                    &other_bone.name,
                                )
                                .changed();
                        }
                    }
                });

            ui.label("Billboard Type");
            changed |= enum_combo_box(ui, id.with("billboard"), &mut bone.billboard_type);
        });
    });

    if let Some(response) = response.completed {
        skel.bones = move_bone(response.from, response.to, &skel.bones);
        changed = true;
    }

    changed
}

fn move_bone(from: usize, to: usize, bones: &[BoneData]) -> Vec<BoneData> {
    // Create a mapping from old indices to new bone indices.
    // This lets us update the bones and parents in one step.
    let mut new_bone_indices: Vec<_> = (0..bones.len()).collect();
    egui_dnd::utils::shift_vec(from, to, &mut new_bone_indices);

    // TODO: Is there a better way to handle invalid parent indices?
    new_bone_indices
        .iter()
        .map(|i| BoneData {
            parent_index: bones[*i]
                .parent_index
                .and_then(|p| new_bone_indices.iter().position(|new_i| *new_i == p)),
            ..bones[*i].clone()
        })
        .collect()
}

fn edit_bones_hierarchy(ui: &mut egui::Ui, skel: &mut SkelData) -> bool {
    let changed = false;

    for (i, bone) in skel.bones.iter().enumerate() {
        if bone.parent_index.is_none() {
            display_bones_recursive(ui, i, &skel.bones);
        }
    }

    changed
}

fn display_bones_recursive(ui: &mut egui::Ui, root_index: usize, bones: &[BoneData]) {
    // TODO: Does this handle cycles?
    // Don't assume bone names are unique.
    let name = &bones[root_index].name;
    let id = ui.make_persistent_id("skel").with(name).with(root_index);

    CollapsingHeader::new(name)
        .id_source(id)
        .default_open(true)
        .show(ui, |ui| {
            // Recursively iterate over the child bones.
            for (i, _) in bones
                .iter()
                .enumerate()
                .filter(|(_, b)| b.parent_index == Some(root_index))
            {
                display_bones_recursive(ui, i, bones);
            }
        });
}

fn match_skel_order(skel: &mut SkelData, reference: &SkelData) {
    // TODO: Sort by helper bones, swing bones, etc for added bones?
    // TODO: This won't correctly handle added bones.
    skel.bones.sort_by_key(|o| {
        // The sort is stable, so unmatched bones will be placed at the end in the same order.
        reference
            .bones
            .iter()
            .position(|r| r.name == o.name)
            .unwrap_or(reference.bones.len())
    })
}

#[cfg(test)]
mod tests {
    use ssbh_data::skel_data::{BillboardType, BoneData};

    use super::*;

    #[test]
    fn skel_order_empty_reference() {
        let mut skel = SkelData {
            major_version: 1,
            minor_version: 0,
            bones: vec![
                BoneData {
                    name: "a".to_owned(),
                    transform: [[0.0; 4]; 4],
                    parent_index: None,
                    billboard_type: BillboardType::Disabled,
                },
                BoneData {
                    name: "b".to_owned(),
                    transform: [[0.0; 4]; 4],
                    parent_index: None,
                    billboard_type: BillboardType::Disabled,
                },
                BoneData {
                    name: "c".to_owned(),
                    transform: [[0.0; 4]; 4],
                    parent_index: None,
                    billboard_type: BillboardType::Disabled,
                },
            ],
        };

        let reference = SkelData {
            major_version: 1,
            minor_version: 0,
            bones: Vec::new(),
        };

        match_skel_order(&mut skel, &reference);

        assert_eq!("a", skel.bones[0].name);
        assert_eq!("b", skel.bones[1].name);
        assert_eq!("c", skel.bones[2].name);
    }

    #[test]
    fn skel_order_added_bonees() {
        let mut skel = SkelData {
            major_version: 1,
            minor_version: 0,
            bones: vec![
                BoneData {
                    name: "a".to_owned(),
                    transform: [[0.0; 4]; 4],
                    parent_index: None,
                    billboard_type: BillboardType::Disabled,
                },
                BoneData {
                    name: "b".to_owned(),
                    transform: [[0.0; 4]; 4],
                    parent_index: None,
                    billboard_type: BillboardType::Disabled,
                },
                BoneData {
                    name: "c".to_owned(),
                    transform: [[0.0; 4]; 4],
                    parent_index: None,
                    billboard_type: BillboardType::Disabled,
                },
            ],
        };

        let reference = SkelData {
            major_version: 1,
            minor_version: 0,
            bones: vec![BoneData {
                name: "c".to_owned(),
                transform: [[0.0; 4]; 4],
                parent_index: None,
                billboard_type: BillboardType::Disabled,
            }],
        };

        match_skel_order(&mut skel, &reference);

        assert_eq!("c", skel.bones[0].name);
        assert_eq!("a", skel.bones[1].name);
        assert_eq!("b", skel.bones[2].name);
    }

    #[test]
    fn swap_bones_same_indices() {
        let before = vec![
            BoneData {
                name: "a".to_owned(),
                transform: [[0.0; 4]; 4],
                parent_index: None,
                billboard_type: BillboardType::Disabled,
            },
            BoneData {
                name: "b".to_owned(),
                transform: [[0.0; 4]; 4],
                parent_index: Some(0),
                billboard_type: BillboardType::Disabled,
            },
        ];

        let after = move_bone(0, 0, &before);
        assert_eq!(before, after);
    }

    #[test]
    fn swap_bones_different_indices() {
        let before = vec![
            BoneData {
                name: "a".to_owned(),
                transform: [[0.0; 4]; 4],
                parent_index: None,
                billboard_type: BillboardType::Disabled,
            },
            BoneData {
                name: "b".to_owned(),
                transform: [[0.0; 4]; 4],
                parent_index: Some(0),
                billboard_type: BillboardType::Disabled,
            },
        ];

        // The target index is 1 higher than expected when moving down.
        let after = move_bone(0, 2, &before);
        assert_eq!(
            vec![
                BoneData {
                    name: "b".to_owned(),
                    transform: [[0.0; 4]; 4],
                    parent_index: Some(1),
                    billboard_type: BillboardType::Disabled,
                },
                BoneData {
                    name: "a".to_owned(),
                    transform: [[0.0; 4]; 4],
                    parent_index: None,
                    billboard_type: BillboardType::Disabled,
                },
            ],
            after
        );
    }

    #[test]
    fn swap_bones_with_parents() {
        let before = vec![
            BoneData {
                name: "a".to_owned(),
                transform: [[0.0; 4]; 4],
                parent_index: Some(1),
                billboard_type: BillboardType::Disabled,
            },
            BoneData {
                name: "b".to_owned(),
                transform: [[0.0; 4]; 4],
                parent_index: Some(0),
                billboard_type: BillboardType::Disabled,
            },
            BoneData {
                name: "c".to_owned(),
                transform: [[0.0; 4]; 4],
                parent_index: Some(0),
                billboard_type: BillboardType::Disabled,
            },
            BoneData {
                name: "d".to_owned(),
                transform: [[0.0; 4]; 4],
                parent_index: Some(2),
                billboard_type: BillboardType::Disabled,
            },
        ];

        // Swap b and c.
        // The bones should still point to the correct parents.
        let after = move_bone(1, 3, &before);
        assert_eq!(
            vec![
                BoneData {
                    name: "a".to_owned(),
                    transform: [[0.0; 4]; 4],
                    parent_index: Some(2),
                    billboard_type: BillboardType::Disabled,
                },
                BoneData {
                    name: "c".to_owned(),
                    transform: [[0.0; 4]; 4],
                    parent_index: Some(0),
                    billboard_type: BillboardType::Disabled,
                },
                BoneData {
                    name: "b".to_owned(),
                    transform: [[0.0; 4]; 4],
                    parent_index: Some(0),
                    billboard_type: BillboardType::Disabled,
                },
                BoneData {
                    name: "d".to_owned(),
                    transform: [[0.0; 4]; 4],
                    parent_index: Some(1),
                    billboard_type: BillboardType::Disabled,
                },
            ],
            after
        );
    }
}
