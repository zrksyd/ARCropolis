// TODO: Share vectors between ssbh_data types?
use ssbh_data::{matl_data::*, meshex_data::Vector4};
use ssbh_wgpu::ShaderProgram;

pub fn load_material_presets<P: AsRef<std::path::Path>>(
    path: P,
) -> Result<Vec<MatlEntryData>, Box<dyn std::error::Error>> {
    let matl: MatlData = serde_json::from_slice(&std::fs::read(path)?)?;
    Ok(matl.entries)
}

pub fn apply_preset(entry: &MatlEntryData, preset: &MatlEntryData) -> MatlEntryData {
    // Textures paths are mesh specific and should be preserved if possible.
    // Remaining textures should use neutral default textures.
    // Preserve the material label to avoid messing up anim and modl data.
    MatlEntryData {
        material_label: entry.material_label.clone(),
        textures: preset
            .textures
            .iter()
            .map(|preset_texture| TextureParam {
                param_id: preset_texture.param_id,
                data: entry
                    .textures
                    .iter()
                    .find(|t| t.param_id == preset_texture.param_id)
                    .map(|t| t.data.clone())
                    .unwrap_or_else(|| default_texture(preset_texture.param_id).to_string()),
            })
            .collect(),
        ..preset.clone()
    }
}

pub fn default_material() -> MatlEntryData {
    // TODO: Make sure the name is unique?
    // TODO: Add defaults for other parameters?
    MatlEntryData {
        material_label: "NEW_MATERIAL".to_string(),
        shader_label: "SFX_PBS_0100000008008269_opaque".to_string(),
        blend_states: vec![BlendStateParam {
            param_id: ParamId::BlendState0,
            data: Default::default(),
        }],
        floats: vec![FloatParam {
            param_id: ParamId::CustomFloat8,
            data: 0.4,
        }],
        booleans: vec![
            BooleanParam {
                param_id: ParamId::CustomBoolean1,
                data: true,
            },
            BooleanParam {
                param_id: ParamId::CustomBoolean3,
                data: true,
            },
            BooleanParam {
                param_id: ParamId::CustomBoolean4,
                data: true,
            },
        ],
        vectors: vec![
            Vector4Param {
                // Set to all zeros to allow for transparency.
                param_id: ParamId::CustomVector0,
                data: Vector4::new(0.0, 0.0, 0.0, 0.0),
            },
            Vector4Param {
                param_id: ParamId::CustomVector13,
                data: Vector4::new(1.0, 1.0, 1.0, 1.0),
            },
            Vector4Param {
                param_id: ParamId::CustomVector14,
                data: Vector4::new(1.0, 1.0, 1.0, 1.0),
            },
            Vector4Param {
                param_id: ParamId::CustomVector8,
                data: Vector4::new(1.0, 1.0, 1.0, 1.0),
            },
        ],
        rasterizer_states: vec![RasterizerStateParam {
            param_id: ParamId::RasterizerState0,
            data: Default::default(),
        }],
        samplers: vec![
            SamplerParam {
                param_id: ParamId::Sampler0,
                data: Default::default(),
            },
            SamplerParam {
                param_id: ParamId::Sampler4,
                data: Default::default(),
            },
            SamplerParam {
                param_id: ParamId::Sampler6,
                data: Default::default(),
            },
            SamplerParam {
                param_id: ParamId::Sampler7,
                data: Default::default(),
            },
        ],
        textures: vec![
            TextureParam {
                param_id: ParamId::Texture0,
                data: default_texture(ParamId::Texture0).to_string(),
            },
            TextureParam {
                param_id: ParamId::Texture4,
                data: default_texture(ParamId::Texture4).to_string(),
            },
            TextureParam {
                param_id: ParamId::Texture6,
                data: default_texture(ParamId::Texture6).to_string(),
            },
            TextureParam {
                param_id: ParamId::Texture7,
                data: default_texture(ParamId::Texture7).to_string(),
            },
        ],
    }
}

pub fn missing_parameters(entry: &MatlEntryData, program: &ShaderProgram) -> Vec<ParamId> {
    program
        .material_parameters
        .iter()
        .copied()
        .filter(|param| {
            !entry
                .booleans
                .iter()
                .map(|p| p.param_id)
                .chain(entry.floats.iter().map(|p| p.param_id))
                .chain(entry.vectors.iter().map(|p| p.param_id))
                .chain(entry.textures.iter().map(|p| p.param_id))
                .chain(entry.samplers.iter().map(|p| p.param_id))
                .chain(entry.blend_states.iter().map(|p| p.param_id))
                .chain(entry.rasterizer_states.iter().map(|p| p.param_id))
                .any(|p| &p == param)
        })
        .collect()
}

pub fn unused_parameters(entry: &MatlEntryData, program: &ShaderProgram) -> Vec<ParamId> {
    entry
        .booleans
        .iter()
        .map(|p| p.param_id)
        .chain(entry.floats.iter().map(|p| p.param_id))
        .chain(entry.vectors.iter().map(|p| p.param_id))
        .chain(entry.textures.iter().map(|p| p.param_id))
        .chain(entry.samplers.iter().map(|p| p.param_id))
        .chain(entry.blend_states.iter().map(|p| p.param_id))
        .chain(entry.rasterizer_states.iter().map(|p| p.param_id))
        .filter(|param| !program.material_parameters.contains(param))
        .collect()
}

pub fn add_parameters(entry: &mut MatlEntryData, parameters: &[ParamId]) {
    // TODO: More intelligently pick defaults
    for param_id in parameters.iter().copied() {
        if is_blend(param_id) {
            entry.blend_states.push(BlendStateParam {
                param_id,
                data: BlendStateData::default(),
            });
        } else if is_float(param_id) {
            entry.floats.push(FloatParam {
                param_id,
                data: 0.0,
            });
        } else if is_bool(param_id) {
            entry.booleans.push(BooleanParam {
                param_id,
                data: false,
            });
        } else if is_vector(param_id) {
            entry.vectors.push(Vector4Param {
                param_id,
                data: Vector4::default(),
            });
        } else if is_rasterizer(param_id) {
            entry.rasterizer_states.push(RasterizerStateParam {
                param_id,
                data: RasterizerStateData::default(),
            });
        } else if is_sampler(param_id) {
            entry.samplers.push(SamplerParam {
                param_id,
                data: SamplerData::default(),
            });
        } else if is_texture(param_id) {
            entry.textures.push(TextureParam {
                param_id,
                data: default_texture(param_id).to_string(),
            });
        }
    }

    // Sort the parameters to match Smash Ultimate's conventions.
    entry.blend_states.sort_by_key(|p| p.param_id as u64);
    entry.floats.sort_by_key(|p| p.param_id as u64);
    entry.booleans.sort_by_key(|p| p.param_id as u64);
    entry.vectors.sort_by_key(|p| p.param_id as u64);
    entry.rasterizer_states.sort_by_key(|p| p.param_id as u64);
    entry.samplers.sort_by_key(|p| p.param_id as u64);
    entry.textures.sort_by_key(|p| p.param_id as u64);
}

pub fn remove_parameters(entry: &mut MatlEntryData, parameters: &[ParamId]) {
    // Using the faster swap_remove function since we sort at the end anyway.
    for param in parameters.iter().copied() {
        if let Some(index) = entry.blend_states.iter().position(|p| p.param_id == param) {
            entry.blend_states.swap_remove(index);
        } else if let Some(index) = entry.floats.iter().position(|p| p.param_id == param) {
            entry.floats.swap_remove(index);
        } else if let Some(index) = entry.booleans.iter().position(|p| p.param_id == param) {
            entry.booleans.swap_remove(index);
        } else if let Some(index) = entry.vectors.iter().position(|p| p.param_id == param) {
            entry.vectors.swap_remove(index);
        } else if let Some(index) = entry
            .rasterizer_states
            .iter()
            .position(|p| p.param_id == param)
        {
            entry.rasterizer_states.swap_remove(index);
        } else if let Some(index) = entry.samplers.iter().position(|p| p.param_id == param) {
            entry.samplers.swap_remove(index);
        } else if let Some(index) = entry.textures.iter().position(|p| p.param_id == param) {
            entry.textures.swap_remove(index);
        }
    }

    // Sort the parameters to match Smash Ultimate's conventions.
    entry.blend_states.sort_by_key(|p| p.param_id as u64);
    entry.floats.sort_by_key(|p| p.param_id as u64);
    entry.booleans.sort_by_key(|p| p.param_id as u64);
    entry.vectors.sort_by_key(|p| p.param_id as u64);
    entry.rasterizer_states.sort_by_key(|p| p.param_id as u64);
    entry.samplers.sort_by_key(|p| p.param_id as u64);
    entry.textures.sort_by_key(|p| p.param_id as u64);
}

// TODO: Move this to ssbh_wgpu?
fn is_vector(p: ParamId) -> bool {
    matches!(
        p,
        ParamId::CustomVector0
            | ParamId::CustomVector1
            | ParamId::CustomVector2
            | ParamId::CustomVector3
            | ParamId::CustomVector4
            | ParamId::CustomVector5
            | ParamId::CustomVector6
            | ParamId::CustomVector7
            | ParamId::CustomVector8
            | ParamId::CustomVector9
            | ParamId::CustomVector10
            | ParamId::CustomVector11
            | ParamId::CustomVector12
            | ParamId::CustomVector13
            | ParamId::CustomVector14
            | ParamId::CustomVector15
            | ParamId::CustomVector16
            | ParamId::CustomVector17
            | ParamId::CustomVector18
            | ParamId::CustomVector19
            | ParamId::CustomVector20
            | ParamId::CustomVector21
            | ParamId::CustomVector22
            | ParamId::CustomVector23
            | ParamId::CustomVector24
            | ParamId::CustomVector25
            | ParamId::CustomVector26
            | ParamId::CustomVector27
            | ParamId::CustomVector28
            | ParamId::CustomVector29
            | ParamId::CustomVector30
            | ParamId::CustomVector31
            | ParamId::CustomVector32
            | ParamId::CustomVector33
            | ParamId::CustomVector34
            | ParamId::CustomVector35
            | ParamId::CustomVector36
            | ParamId::CustomVector37
            | ParamId::CustomVector38
            | ParamId::CustomVector39
            | ParamId::CustomVector40
            | ParamId::CustomVector41
            | ParamId::CustomVector42
            | ParamId::CustomVector43
            | ParamId::CustomVector44
            | ParamId::CustomVector45
            | ParamId::CustomVector46
            | ParamId::CustomVector47
            | ParamId::CustomVector48
            | ParamId::CustomVector49
            | ParamId::CustomVector50
            | ParamId::CustomVector51
            | ParamId::CustomVector52
            | ParamId::CustomVector53
            | ParamId::CustomVector54
            | ParamId::CustomVector55
            | ParamId::CustomVector56
            | ParamId::CustomVector57
            | ParamId::CustomVector58
            | ParamId::CustomVector59
            | ParamId::CustomVector60
            | ParamId::CustomVector61
            | ParamId::CustomVector62
            | ParamId::CustomVector63
    )
}

fn is_rasterizer(p: ParamId) -> bool {
    matches!(
        p,
        ParamId::RasterizerState0
            | ParamId::RasterizerState1
            | ParamId::RasterizerState2
            | ParamId::RasterizerState3
            | ParamId::RasterizerState4
            | ParamId::RasterizerState5
            | ParamId::RasterizerState6
            | ParamId::RasterizerState7
            | ParamId::RasterizerState8
            | ParamId::RasterizerState9
            | ParamId::RasterizerState10
    )
}

fn is_blend(p: ParamId) -> bool {
    matches!(
        p,
        ParamId::BlendState0
            | ParamId::BlendState1
            | ParamId::BlendState2
            | ParamId::BlendState3
            | ParamId::BlendState4
            | ParamId::BlendState5
            | ParamId::BlendState6
            | ParamId::BlendState7
            | ParamId::BlendState8
            | ParamId::BlendState9
            | ParamId::BlendState10
    )
}

fn is_float(p: ParamId) -> bool {
    matches!(
        p,
        ParamId::CustomFloat0
            | ParamId::CustomFloat1
            | ParamId::CustomFloat2
            | ParamId::CustomFloat3
            | ParamId::CustomFloat4
            | ParamId::CustomFloat5
            | ParamId::CustomFloat6
            | ParamId::CustomFloat7
            | ParamId::CustomFloat8
            | ParamId::CustomFloat9
            | ParamId::CustomFloat10
            | ParamId::CustomFloat11
            | ParamId::CustomFloat12
            | ParamId::CustomFloat13
            | ParamId::CustomFloat14
            | ParamId::CustomFloat15
            | ParamId::CustomFloat16
            | ParamId::CustomFloat17
            | ParamId::CustomFloat18
            | ParamId::CustomFloat19
    )
}

fn is_texture(p: ParamId) -> bool {
    matches!(
        p,
        ParamId::Texture0
            | ParamId::Texture1
            | ParamId::Texture2
            | ParamId::Texture3
            | ParamId::Texture4
            | ParamId::Texture5
            | ParamId::Texture6
            | ParamId::Texture7
            | ParamId::Texture8
            | ParamId::Texture9
            | ParamId::Texture10
            | ParamId::Texture11
            | ParamId::Texture12
            | ParamId::Texture13
            | ParamId::Texture14
            | ParamId::Texture15
            | ParamId::Texture16
            | ParamId::Texture17
            | ParamId::Texture18
            | ParamId::Texture19
    )
}

fn is_sampler(p: ParamId) -> bool {
    matches!(
        p,
        ParamId::Sampler0
            | ParamId::Sampler1
            | ParamId::Sampler2
            | ParamId::Sampler3
            | ParamId::Sampler4
            | ParamId::Sampler5
            | ParamId::Sampler6
            | ParamId::Sampler7
            | ParamId::Sampler8
            | ParamId::Sampler9
            | ParamId::Sampler10
            | ParamId::Sampler11
            | ParamId::Sampler12
            | ParamId::Sampler13
            | ParamId::Sampler14
            | ParamId::Sampler15
            | ParamId::Sampler16
            | ParamId::Sampler17
            | ParamId::Sampler18
            | ParamId::Sampler19
    )
}

fn is_bool(p: ParamId) -> bool {
    matches!(
        p,
        ParamId::CustomBoolean0
            | ParamId::CustomBoolean1
            | ParamId::CustomBoolean2
            | ParamId::CustomBoolean3
            | ParamId::CustomBoolean4
            | ParamId::CustomBoolean5
            | ParamId::CustomBoolean6
            | ParamId::CustomBoolean7
            | ParamId::CustomBoolean8
            | ParamId::CustomBoolean9
            | ParamId::CustomBoolean10
            | ParamId::CustomBoolean11
            | ParamId::CustomBoolean12
            | ParamId::CustomBoolean13
            | ParamId::CustomBoolean14
            | ParamId::CustomBoolean15
            | ParamId::CustomBoolean16
            | ParamId::CustomBoolean17
            | ParamId::CustomBoolean18
            | ParamId::CustomBoolean19
    )
}

fn default_texture(p: ParamId) -> &'static str {
    // The default texture should have as close as possible to no effect.
    // This reduces the number of textures that need to be manually assigned.
    match p {
        ParamId::Texture0 => "/common/shader/sfxpbs/default_white",
        ParamId::Texture1 => "/common/shader/sfxpbs/default_white",
        ParamId::Texture2 => "#replace_cubemap",
        ParamId::Texture3 => "/common/shader/sfxpbs/default_white",
        ParamId::Texture4 => "/common/shader/sfxpbs/fighter/default_normal",
        ParamId::Texture5 => "/common/shader/sfxpbs/default_black",
        ParamId::Texture6 => "/common/shader/sfxpbs/fighter/default_params",
        ParamId::Texture7 => "#replace_cubemap",
        ParamId::Texture8 => "#replace_cubemap", // TODO: Better default cube map?
        ParamId::Texture9 => "/common/shader/sfxpbs/default_black",
        ParamId::Texture10 => "/common/shader/sfxpbs/default_white",
        ParamId::Texture11 => "/common/shader/sfxpbs/default_white",
        ParamId::Texture12 => "/common/shader/sfxpbs/default_white",
        ParamId::Texture13 => "/common/shader/sfxpbs/default_white",
        ParamId::Texture14 => "/common/shader/sfxpbs/default_black",
        ParamId::Texture15 => "/common/shader/sfxpbs/default_white",
        ParamId::Texture16 => "/common/shader/sfxpbs/default_white",
        ParamId::Texture17 => "/common/shader/sfxpbs/default_white",
        ParamId::Texture18 => "/common/shader/sfxpbs/default_white",
        ParamId::Texture19 => "/common/shader/sfxpbs/default_white",
        _ => "/common/shader/sfxpbs/default_white",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_parameters_all_missing() {
        let mut entry = MatlEntryData {
            material_label: String::new(),
            shader_label: String::new(),
            blend_states: Vec::new(),
            floats: Vec::new(),
            booleans: Vec::new(),
            vectors: Vec::new(),
            rasterizer_states: Vec::new(),
            samplers: Vec::new(),
            textures: Vec::new(),
        };

        let required_parameters = missing_parameters(
            &entry,
            &ShaderProgram {
                discard: false,
                vertex_attributes: Vec::new(),
                material_parameters: vec![
                    ParamId::BlendState0,
                    ParamId::CustomFloat0,
                    ParamId::CustomBoolean0,
                    ParamId::CustomVector0,
                    ParamId::RasterizerState0,
                    ParamId::Sampler0,
                    ParamId::Texture0,
                ],
            },
        );
        add_parameters(&mut entry, &required_parameters);

        // TODO: Add tests for better default values.
        assert_eq!(
            MatlEntryData {
                material_label: String::new(),
                shader_label: String::new(),
                blend_states: vec![BlendStateParam {
                    param_id: ParamId::BlendState0,
                    data: Default::default(),
                }],
                floats: vec![FloatParam {
                    param_id: ParamId::CustomFloat0,
                    data: Default::default(),
                }],
                booleans: vec![BooleanParam {
                    param_id: ParamId::CustomBoolean0,
                    data: Default::default(),
                }],
                vectors: vec![Vector4Param {
                    param_id: ParamId::CustomVector0,
                    data: Default::default(),
                }],
                rasterizer_states: vec![RasterizerStateParam {
                    param_id: ParamId::RasterizerState0,
                    data: Default::default(),
                }],
                samplers: vec![SamplerParam {
                    param_id: ParamId::Sampler0,
                    data: Default::default(),
                }],
                textures: vec![TextureParam {
                    param_id: ParamId::Texture0,
                    data: "/common/shader/sfxpbs/default_white".to_string(),
                }],
            },
            entry
        );
    }

    #[test]
    fn remove_parameters_all_unused() {
        let mut entry = MatlEntryData {
            material_label: String::new(),
            shader_label: String::new(),
            blend_states: vec![BlendStateParam {
                param_id: ParamId::BlendState0,
                data: Default::default(),
            }],
            floats: vec![FloatParam {
                param_id: ParamId::CustomFloat0,
                data: Default::default(),
            }],
            booleans: vec![BooleanParam {
                param_id: ParamId::CustomBoolean0,
                data: Default::default(),
            }],
            vectors: vec![Vector4Param {
                param_id: ParamId::CustomVector0,
                data: Default::default(),
            }],
            rasterizer_states: vec![RasterizerStateParam {
                param_id: ParamId::RasterizerState0,
                data: Default::default(),
            }],
            samplers: vec![SamplerParam {
                param_id: ParamId::Sampler0,
                data: Default::default(),
            }],
            textures: vec![TextureParam {
                param_id: ParamId::Texture0,
                data: Default::default(),
            }],
        };

        let unused_parameters = unused_parameters(
            &entry,
            &ShaderProgram {
                discard: false,
                vertex_attributes: Vec::new(),
                material_parameters: Vec::new(),
            },
        );
        remove_parameters(&mut entry, &unused_parameters);

        assert!(entry.blend_states.is_empty());
        assert!(entry.floats.is_empty());
        assert!(entry.booleans.is_empty());
        assert!(entry.vectors.is_empty());
        assert!(entry.rasterizer_states.is_empty());
        assert!(entry.samplers.is_empty());
        assert!(entry.textures.is_empty());
    }

    #[test]
    fn apply_preset_empty_material() {
        let mut entry = MatlEntryData {
            material_label: "material".to_string(),
            shader_label: "123".to_string(),
            blend_states: Vec::new(),
            floats: Vec::new(),
            booleans: Vec::new(),
            vectors: Vec::new(),
            rasterizer_states: Vec::new(),
            samplers: Vec::new(),
            textures: vec![TextureParam {
                param_id: ParamId::Texture0,
                data: "a".to_string(),
            }],
        };

        let preset = MatlEntryData {
            material_label: "preset".to_string(),
            shader_label: "456".to_string(),
            blend_states: vec![BlendStateParam {
                param_id: ParamId::BlendState0,
                data: Default::default(),
            }],
            floats: vec![FloatParam {
                param_id: ParamId::CustomFloat0,
                data: Default::default(),
            }],
            booleans: vec![BooleanParam {
                param_id: ParamId::CustomBoolean0,
                data: Default::default(),
            }],
            vectors: vec![Vector4Param {
                param_id: ParamId::CustomVector0,
                data: Default::default(),
            }],
            rasterizer_states: vec![RasterizerStateParam {
                param_id: ParamId::RasterizerState0,
                data: Default::default(),
            }],
            samplers: vec![
                SamplerParam {
                    param_id: ParamId::Sampler0,
                    data: Default::default(),
                },
                SamplerParam {
                    param_id: ParamId::Sampler1,
                    data: Default::default(),
                },
            ],
            textures: vec![
                TextureParam {
                    param_id: ParamId::Texture0,
                    data: "d".to_string(),
                },
                TextureParam {
                    param_id: ParamId::Texture1,
                    data: "c".to_string(),
                },
            ],
        };

        entry = apply_preset(&entry, &preset);

        assert_eq!(
            MatlEntryData {
                material_label: "material".to_string(),
                shader_label: "456".to_string(),
                blend_states: vec![BlendStateParam {
                    param_id: ParamId::BlendState0,
                    data: Default::default(),
                }],
                floats: vec![FloatParam {
                    param_id: ParamId::CustomFloat0,
                    data: Default::default(),
                }],
                booleans: vec![BooleanParam {
                    param_id: ParamId::CustomBoolean0,
                    data: Default::default(),
                }],
                vectors: vec![Vector4Param {
                    param_id: ParamId::CustomVector0,
                    data: Default::default(),
                }],
                rasterizer_states: vec![RasterizerStateParam {
                    param_id: ParamId::RasterizerState0,
                    data: Default::default(),
                }],
                samplers: vec![
                    SamplerParam {
                        param_id: ParamId::Sampler0,
                        data: Default::default(),
                    },
                    SamplerParam {
                        param_id: ParamId::Sampler1,
                        data: Default::default(),
                    }
                ],
                textures: vec![
                    TextureParam {
                        param_id: ParamId::Texture0,
                        data: "a".to_string(),
                    },
                    TextureParam {
                        param_id: ParamId::Texture1,
                        data: "/common/shader/sfxpbs/default_white".to_string(),
                    }
                ],
            },
            entry
        );
    }
}