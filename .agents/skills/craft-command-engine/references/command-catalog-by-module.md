# PhotoCraft Command Catalog by Module

All engine commands are registered in `crates/engine/src/commands.rs::build()`, which combines built-in specs with `specs()` vectors from ~85 focused command modules in `crates/engine/src/`. Shell and tool commands live in `crates/engine/src/shell_commands.rs`.

---

## 1. File, Import/Export, Presets & Automation

| Module (`crates/engine/src/`) | Key Command IDs & Responsibilities |
|---|---|
| `commands.rs` | `file.new`, `file.close` |
| `file_cmds.rs` | `file.open`, `file.openAs`, `file.openAsSmartObject`, `file.save`, `file.saveAs`, `file.saveACopy`, `file.revert`, `file.closeAll`, `file.closeOthers`, `file.placeEmbedded`, `file.placeLinked`, `file.package`, `file.fileInfo`, `file.export.*` (layersToFiles, pathsToIllustrator, lut, dataSetsAsFiles, zoomify, artboardsToFiles/Pdf) |
| `exr_cmds.rs` | OpenEXR multi-channel / layer import & export options |
| `web_cmds.rs` | `file.export.saveForWeb` (Save for Web (Legacy) optimizer, slice export, GIF/PNG-8/JPEG/WBMP) |
| `print_cmds.rs` | `file.print`, `file.printOneCopy`, `file.pageSetup` (CUPS/IPP/portal spooling, color management, print marks) |
| `wia_cmds.rs` | `file.import.wiaSupport`, scanner/image acquisition |
| `automate_cmds.rs` | `file.automate.batch`, `file.automate.pdfPresentation`, `file.automate.createDroplet`, `file.automate.cropAndStraightenPhotos`, `file.automate.fitImage`, `file.automate.conditionalModeChange`, `file.scripts.*` (imageProcessor, scriptEventsManager, deleteAllEmptyLayers, flattenAllLayerEffects, flattenAllMasks, browse) |
| `photo_cmds.rs` | `file.automate.contactSheetII`, `file.automate.photomerge`, `file.automate.mergeToHdrPro`, `file.automate.lensCorrection`, `file.scripts.loadFilesIntoStack`, `file.scripts.statistics` |
| `document_preset_cmds.rs` | `file.new` preset validation, saved document size presets (`documentPresets.*`) |
| `export_preset_cmds.rs` | Export As / Quick Export format preferences and presets |
| `actions_cmds.rs` & `actions_edit_cmds.rs` | Window › Actions (`actions.newSet`, `actions.new`, `actions.record`, `actions.stop`, `actions.play`, `actions.delete`, `actions.duplicate`, step insertion/reordering/toggle) |
| `atn_cmds.rs` | `.atn` Photoshop Actions file import/export (`actions.load`, `actions.save`) |
| `presets.rs` & `preset_import_cmds.rs` | `edit.presetManager`, preset libraries for Gradients, Patterns, Styles, Shapes, Tool Presets, Clone Source, and importing/exporting `.abr`, `.aco`, `.ase`, `.act`, `.grd`, `.pat`, `.asl`, `.csh`, `.tpl` |
| `migrate_cmds.rs` | `edit.presets.migrate`, `edit.presets.exportImport` |

---

## 2. Edit, History, Clipboard, Transform & Preferences

| Module (`crates/engine/src/`) | Key Command IDs & Responsibilities |
|---|---|
| `commands.rs` | `edit.undo`, `edit.redo`, `edit.fill`, `edit.clear` |
| `edit_cmds.rs` | `edit.cut`, `edit.copy`, `edit.copyMerged`, `edit.paste`, `edit.pasteSpecial.pasteInPlace`, `edit.pasteSpecial.pasteInto`, `edit.pasteSpecial.pasteOutside`, `edit.stroke`, `edit.defineBrushPreset`, `edit.definePattern` |
| `edit_menu_cmds.rs` | `edit.fade`, `edit.contentAwareScale`, `edit.defineCustomShape`, `edit.purge.*` (undo, clipboard, histories, all, videoCache) |
| `fill_cmds.rs` & `fill_key_cmds.rs` | `edit.fill` implementation (foreground, background, color, content-aware PatchMatch, pattern, history, black, 50% gray, white) and shortcut fills (`edit.fillForeground`, `edit.fillBackground`, including preserve-transparency variants) |
| `history_cmds.rs`, `history_menu_cmds.rs`, `history_source_cmds.rs`, `history_select_cmds.rs` | `edit.stepForward`, `edit.stepBackward`, `history.newSnapshot`, `history.deleteSnapshot`, `history.newDocument`, `history.clear`, `history.setSource`, `history.select` |
| `transform_cmds.rs` | `edit.transform.*` (`again`, `againCopy`, `scale`, `rotate`, `skew`, `distort`, `perspective`, `rotate180`, `rotate90cw`, `rotate90ccw`, `flipHorizontal`, `flipVertical`, free transform matrix/quad application) |
| `warp_cmds.rs` | `edit.transform.warp`, `edit.perspectiveWarp`, `edit.puppetWarp` |
| `align_cmds.rs` | `edit.autoAlignLayers`, `edit.autoBlendLayers` |
| `color_cmds.rs` & `installed_profiles.rs` | `edit.colorSettings`, `edit.assignProfile`, `edit.convertToProfile`, `color.installedProfiles` |
| `prefs.rs` | `edit.preferences.*`, `edit.keyboardShortcuts`, `edit.menus`, `edit.toolbar` |

---

## 3. Image, Color Modes, Adjustments, Variables & Analysis

| Module (`crates/engine/src/`) | Key Command IDs & Responsibilities |
|---|---|
| `commands.rs` | `image.imageRotation.*` (`180`, `90cw`, `90ccw`, `flipCanvasHorizontal`, `flipCanvasVertical`), `image.adjustments.desaturate`, plus the `ADJ` table generating pairs of `image.adjustments.<kind>` and `layer.newAdjustmentLayer.<kind>` for all 16 core adjustments (`brightnessContrast`, `levels`, `curves`, `exposure`, `vibrance`, `hueSaturation`, `colorBalance`, `blackWhite`, `photoFilter`, `channelMixer`, `colorLookup`, `invert`, `posterize`, `threshold`, `gradientMap`, `selectiveColor`) |
| `adjust_cmds.rs` & `adjust_params.rs` | `image.adjustments.shadowsHighlights`, `hdrToning`, `matchColor`, `replaceColor`, `equalize`, `variations`, `autoTone`, `autoContrast`, `autoColor`, Smart Filter adjustment recording, and parameter/eyedropper parsing |
| `image_cmds.rs` | `image.imageSize`, `image.canvasSize`, `image.imageRotation.arbitrary`, `image.crop`, `image.trim`, `image.revealAll`, `image.duplicate`, `image.applyImage`, `image.calculations` |
| `perspective_crop_cmds.rs` | `image.perspectiveCrop` (non-axis-aligned quad perspective rectification) |
| `mode_cmds.rs` & `multichannel_cmds.rs` | `image.mode.*` (`bitmap`, `grayscale`, `duotone`, `indexedColor`, `rgb`, `cmyk`, `lab`, `multichannel`, `8bits`, `16bits`, `32bits`, `colorTable`) |
| `trap_cmds.rs` | `image.trap` (CMYK print trapping) |
| `variables_cmds.rs` | `image.variables.define`, `image.variables.dataSets`, `image.applyDataSet` |
| `analysis_cmds.rs` | `image.analysis.*` (`setMeasurementScale`, `selectDataPoints`, `recordMeasurements`, `rulerTool`, `countTool`, `placeScaleMarker`) |

---

## 4. Layers, Masks, Styles, Smart Objects, Artboards & Comps

| Module (`crates/engine/src/`) | Key Command IDs & Responsibilities |
|---|---|
| `commands.rs` & `new_layer.rs` | `layer.new.layer`, `layer.new.group`, `layer.groupLayers`, `layer.duplicate`, `layer.delete`, `layer.select`, `layer.setProps`, `layer.arrange.*`, `layer.createClippingMask`, `layer.releaseClippingMask`, `layer.layerMask.*` (`revealAll`, `hideAll`, `revealSelection`, `delete`), `layer.mergeDown`, `layer.mergeGroup`, `layer.flattenImage`, `layer.newFillLayer.solidColor`, `layer.newFillLayer.gradient`, `layer.setAdjustment`, `layer.moveTo`, `layer.translate` |
| `background_cmds.rs` & `extra_cmds.rs` | `layer.new.layerFromBackground`, `layer.new.backgroundFromLayer`, `layer.mergeVisible`, `layer.stampVisible` |
| `layer_menu_cmds.rs` | `layer.renameLayer`, `layer.delete.hiddenLayers`, `layer.QuickExportAsPng`, `layer.exportAs`, `layer.ungroupLayers`, `layer.hideLayers`, `layer.showLayers`, `layer.align.*`, `layer.distribute.*`, `layer.lockLayers`, `layer.linkLayers`, `layer.selectLinkedLayers`, `layer.combineShapes.*`, `layer.matting.*` (`colorDecontaminate`, `defringe`, `removeBlackMatte`, `removeWhiteMatte`) |
| `layer_multi_cmds.rs` & `layer_copy_cmds.rs` | Multi-layer selection, grouping, duplication (`layer.new.layerViaCopy`, `layer.new.layerViaCut`, `layer.copyToDocument`), deletion, and pixel damage calculation |
| `layer_list_cmds.rs`, `layer_nav_cmds.rs`, `layer_label_cmds.rs`, `group_view_cmds.rs` | Layers panel filtering/searching, keyboard layer navigation (`layer.selectNext`, `layer.selectPrevious`, `layer.selectTop`, `layer.selectBottom`), color labels (`layer.setColor`), group expand/collapse (`layer.setGroupExpanded`, `layer.showOnly`) |
| `solid_fill_cmds.rs`, `gradient_fill_cmds.rs`, `pattern_fill_cmds.rs`, `pattern_cmds.rs` | Fill layer creation/editing (`layer.newFillLayer.pattern`, `layer.setFill`) and pattern library management |
| `layer_mask_props_cmds.rs` & `mask_view_cmds.rs` | `layer.layerMask.fromTransparency`, `hideSelection`, `apply`, `enable`, `link`, mask density/feather (`layer.setMaskProps`), rubylith/solo mask view modes |
| `layer_style.rs`, `rasterize_style_cmds.rs`, `fx_view_cmds.rs`, `fx_visibility_cmds.rs` | `layer.layerStyle.*` (blendingOptions, dropShadow, innerShadow, outerGlow, innerGlow, bevelAndEmboss, satin, colorOverlay, gradientOverlay, patternOverlay, stroke, copyLayerStyle, pasteLayerStyle, clearLayerStyle, createLayers, scaleEffects, hideAllEffects) and `layer.rasterize.*` |
| `smart_cmds.rs` | `layer.smartObjects.*` (`convertToSmartObject`, `newSmartObjectViaCopy`, `editContents`, `replaceContents`, `exportContents`, `embedLinked`, `relinkToFile`, `updateModifiedContent`, `resetTransform`, `rasterize`, `stackMode.*`), `layer.smartFilter.*` |
| `artboard_cmds.rs` & `frame_cmds.rs` | `layer.new.artboard`, `layer.new.artboardFromGroup`, `layer.new.artboardFromLayers`, `layer.artboard.*`, `layer.new.frame`, `layer.frame.*` |
| `comps_cmds.rs` | Window › Layer Comps (`comps.new`, `comps.apply`, `comps.update`, `comps.delete`, `comps.duplicate`, `comps.rename`, `comps.next`, `comps.previous`) |

---

## 5. Type, Selections, Channels, Vectors & Filters

| Module (`crates/engine/src/`) | Key Command IDs & Responsibilities |
|---|---|
| `type_cmds.rs`, `type_extra_cmds.rs`, `type_mask_cmds.rs`, `type_styles_cmds.rs`, `type_spell_cmds.rs`, `type_caret_cmds.rs` | `type.create`, `type.edit`, `type.orientation.*`, `type.antiAlias.*`, `type.convertToPointText`, `type.convertToParagraphText`, `type.convertToShape`, `type.createWorkPath`, `type.extrudeTo3D`, `type.warpText`, `type.matchFont`, `type.fontPreviewSize.*`, `type.languageOptions.*`, `type.updateAllTextLayers`, `type.replaceAllMissingFonts`, `type.pasteLoremIpsum`, `type.loadDefaultTypeStyles`, `type.saveDefaultTypeStyles`, `type.characterStyles.*`, `type.paragraphStyles.*`, `edit.checkSpelling`, `edit.findAndReplaceText`, on-canvas caret editing |
| `selection_cmds.rs`, `select_extra_cmds.rs`, `smartselect_cmds.rs`, `magnetic_cmds.rs`, `float_cmds.rs` | `select.all`, `select.deselect`, `select.reselect`, `select.inverse`, `select.allLayers`, `select.deselectLayers`, `select.findLayers`, `select.isolateLayers`, `select.colorRange`, `select.focusArea`, `select.subject`, `select.sky`, `select.selectAndMask`, `select.modify.*` (`border`, `smooth`, `expand`, `contract`, `feather`), `select.grow`, `select.similar`, `select.transformSelection`, `select.loadSelection`, `select.saveSelection`, `select.new3DExtrusion`, `select.rect`, `select.lasso`, `select.polygonLasso`, `select.magneticLasso`, `select.magicWand`, `select.quickSelection`, `select.objectSelection`, `select.float`, `select.drop` |
| `channel_cmds.rs` | `channel.new`, `channel.duplicate`, `channel.delete`, `channel.select`, `channel.setVisible`, `channel.setProps`, `channel.splitChannels`, `channel.mergeChannels`, `channel.newSpotChannel`, `select.editInQuickMaskMode` |
| `vector_cmds.rs` & `path_edit_cmds.rs` | `path.setWork`, `path.save`, `path.delete`, `path.duplicate`, `path.rename`, `path.toSelection`, `path.fromSelection`, `path.fill`, `path.stroke`, `path.clippingPath`, anchor point editing, `shape.create`, `shape.setStyle`, `layer.vectorMask.*` |
| `filters.rs` & `filters_ext.rs` | `filter.lastFilter`, `filter.convertForSmartFilters`, `filter.blur.*`, `filter.blurGallery.*`, `filter.distort.*`, `filter.noise.*`, `filter.pixelate.*`, `filter.render.*`, `filter.sharpen.*`, `filter.stylize.*`, `filter.video.*`, `filter.other.*` |
| `gallery_cmds.rs`, `distort_cmds.rs`, `lens_cmds.rs`, `vp_cmds.rs`, `render_cmds.rs`, `cutout_cmds.rs`, `face_cmds.rs`, `color_to_alpha_cmds.rs` | `filter.filterGallery` (Artistic, Brush Strokes, Distort, Sketch, Stylize, Texture), `filter.liquify` (including Face-Aware Liquify in `face_cmds.rs`), `filter.wideAngle`, `filter.cameraRaw`, `filter.lensCorrection`, `filter.vanishingPoint`, procedural `Flame` / `Picture Frame` / `Tree` / `Lighting Effects`, `Cutout`, `Color to Alpha` |
| `plugin_cmds.rs` | Sandboxed WebAssembly plug-ins (`plugin.install`, `plugin.run`, `plugin.list`, `plugin.remove`) |

---

## 6. Paint, Retouch, Timeline, View State & Non-Journaled Queries

| Module (`crates/engine/src/`) | Key Command IDs & Responsibilities |
|---|---|
| `brush_cmds.rs`, `brush_preset_cmds.rs`, `brush_key_cmds.rs`, `swatch_cmds.rs`, `symmetry_cmds.rs` | `paint.stroke`, `paint.pencil`, `paint.mixerBrush`, `paint.colorReplacement`, `tools.setBrush`, `tools.setColors`, `tools.swapColors`, `tools.defaultColors`, `brush.presets.*`, bracket-key size/hardness/opacity shortcuts, `swatches.*`, `symmetry.*` |
| `paint_cmds.rs`, `eraser_cmds.rs`, `retouch_cmds.rs`, `stamp_cmds.rs`, `redeye_cmds.rs`, `sample_cmds.rs`, `pick_cmds.rs` | `paint.gradient`, `paint.bucket`, `paint.erase`, `paint.backgroundEraser`, `paint.magicEraser`, `paint.spotHealing`, `paint.healing`, `paint.patch`, `paint.contentAwareMove`, `paint.remove`, `paint.dodge`, `paint.burn`, `paint.sponge`, `paint.blurTool`, `paint.sharpenTool`, `paint.smudge`, `paint.cloneStamp`, `paint.patternStamp`, `paint.historyBrush`, `paint.artHistoryBrush`, `paint.redEye`, `tools.eyedropper`, `select.layerAtPoint` |
| `timeline_cmds.rs` & `video_cmds.rs` | `timeline.*` (frame animation & video timeline tracks, keyframes, onion skin, playback), `file.import.videoFramesToLayers`, `file.export.renderVideo` |
| `proof_sim.rs`, `sampler_cmds.rs`, `notes_cmds.rs`, `slice_cmds.rs`, `guide_layout.rs` | `view.proofSetup`, `view.proofColors`, `view.gamutWarning`, `view.pixelAspectRatio.*`, `view.guides.*` (`newGuide`, `newGuideLayout`, `newGuidesFromShape`, `clearGuides`, `clearCanvasGuides`), `view.snapTo.*`, `view.colorSamplers.*`, `notes.*`, `slice.*` |
| `commands.rs` & `jobs.rs` | **Non-journaled queries (`journal: false`):** `session.inspect`, `document.inspect`, `document.activate`, `document.move`, `document.pixel`, `command.list`, `job.status`, `job.cancel` |

---

## 7. Shell & Tool Commands (`crates/engine/src/shell_commands.rs`)

- **`SHELL_COMMANDS` (`ui-egui/src/menus.rs::UI_COMMANDS`):**
  - **View & Zoom:** `view.zoomIn`, `view.zoomOut`, `view.fitOnScreen`, `view.actualPixels`, `view.rotateView`, `view.resetView`, `view.rulers`, `view.snap`, `view.lockGuides`, `view.extras`, `view.show.grid`, `view.show.guides`, `view.show.targetPath`, `view.screenMode.cycle`, `view.darkenInterface`, `view.lightenInterface`
  - **Panels & Windows:** `window.toggle.*` (`layers`, `history`, `properties`, `color`, `brushSettings`, `brushes`, `navigator`, `toolbar`, `options`, `dock`), `window.togglePanels`, `window.panel.*` (`group`, `layout`, `move`, `float`, `dock`, `activate`, `close`), `window.newWindowForDocument`, `window.nextDocument`, `window.previousDocument`, `window.theme.*`
  - **Interactive Shell Modal Triggers:** `file.open`, `file.save`, `file.saveAs`, `file.exit`, `file.export.exportAs`, `file.export.quickExportAsPng`, `edit.freeTransform`, `edit.freeTransformCopy`, `edit.transform.*`, `edit.search`, `select.selectAndMask`, `type.editText`, `ui.symmetryTransform`, `tool.reset`, `tool.resetAll`, `help.*`
- **`TOOLS` (67 toolbar tools, `tools.select.<tool>`):**
  - `move`, `artboard`, `rectMarquee`, `ellipseMarquee`, `singleRowMarquee`, `singleColumnMarquee`, `lasso`, `polygonLasso`, `magneticLasso`, `magicWand`, `quickSelection`, `objectSelection`, `crop`, `perspectiveCrop`, `slice`, `sliceSelect`, `frame`, `eyedropper`, `colorSampler`, `ruler`, `note`, `count`, `spotHealing`, `healing`, `patch`, `contentAwareMove`, `redEye`, `remove`, `brush`, `pencil`, `colorReplacement`, `mixerBrush`, `cloneStamp`, `patternStamp`, `historyBrush`, `eraser`, `backgroundEraser`, `magicEraser`, `gradient`, `paintBucket`, `blur`, `sharpen`, `smudge`, `dodge`, `burn`, `sponge`, `pen`, `freeformPen`, `curvaturePen`, `addAnchorPoint`, `deleteAnchorPoint`, `convertPoint`, `type`, `verticalType`, `horizontalTypeMask`, `verticalTypeMask`, `pathSelection`, `directSelection`, `rectangle`, `ellipseShape`, `triangle`, `polygon`, `line`, `customShape`, `hand`, `rotateView`, `zoom`.
