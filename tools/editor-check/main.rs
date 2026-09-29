//! Disposable, silent GUI test driver using foxdaw's native VST3 host unchanged.
use eframe::egui;
use foxdaw_vst3_host::{
    Checkpoint, Descriptor, DeviceId, DeviceKey, NativeGraph, ProcessCapability,
};
use std::{path::PathBuf, time::Duration};

struct Check {
    descriptor: Descriptor,
    graphs: Vec<Option<(NativeGraph, ProcessCapability)>>,
    commands: PathBuf,
    consumed: usize,
    incarnation: u64,
    status: String,
    retired: bool,
}
impl Check {
    fn load(&mut self, index: usize, checkpoint: &Checkpoint) {
        self.incarnation += 1;
        let mut graph = NativeGraph::load_with_role(
            &self.descriptor,
            self.descriptor.role,
            DeviceKey {
                device: DeviceId(index as u64 + 1),
                incarnation: self.incarnation,
            },
            checkpoint,
            48000,
        )
        .unwrap();
        assert!(graph.poll_ready().unwrap());
        assert!(graph.restore_diagnostic().is_none());
        let capability = graph.issue_process().unwrap();
        self.graphs[index] = Some((graph, capability));
    }
    fn snapshot(&mut self, label: &str) {
        let root = PathBuf::from(std::env::var_os("FOXDAW_NATIVE_ARTIFACTS").unwrap());
        let mut states = Vec::new();
        for (index, pair) in self.graphs.iter_mut().enumerate() {
            let Some((graph, capability)) = pair else {
                continue;
            };
            // Flush GUI edits and processor feedback before reading controller values.
            let state = graph.capture(capability).unwrap();
            let info = graph.parameters().to_vec();
            let parameters: Vec<_> = info.iter().filter(|p| p.automatable).map(|p| {
                serde_json::json!({"id":p.id,"name":p.name,"value":graph.parameter_value(p.id).unwrap()})
            }).collect();
            states.push(serde_json::json!({"instance":index,"parameters":parameters,
                "component":state.component.as_ref(),"controller":state.controller.as_ref()}));
        }
        std::fs::write(
            root.join(format!("{label}.json")),
            serde_json::to_vec_pretty(&states).unwrap(),
        )
        .unwrap();
        println!("snapshot {label}: {} instances", states.len());
    }
    fn execute(&mut self, command: &str) {
        let parts: Vec<_> = command.split_whitespace().collect();
        let index = parts
            .get(1)
            .and_then(|p| p.parse::<usize>().ok())
            .unwrap_or(0);
        match parts[0] {
            "open" => self.graphs[index]
                .as_mut()
                .unwrap()
                .0
                .open_editor()
                .unwrap(),
            "close" => self.graphs[index]
                .as_mut()
                .unwrap()
                .0
                .close_editor()
                .unwrap(),
            "snapshot" => self.snapshot(parts[1]),
            "display" => {
                let id = parts[2].parse().unwrap();
                let value = parts[3].parse().unwrap();
                self.graphs[index]
                    .as_mut()
                    .unwrap()
                    .0
                    .automation_display(&[(id, value)])
                    .unwrap();
            }
            "recreate" => {
                let (mut graph, mut capability) = self.graphs[index].take().unwrap();
                let checkpoint = graph.capture(&mut capability).unwrap();
                graph
                    .retire(capability)
                    .unwrap_or_else(|e| panic!("retirement: {e}"));
                self.load(index, &checkpoint);
                self.graphs[index]
                    .as_mut()
                    .unwrap()
                    .0
                    .open_editor()
                    .unwrap();
            }
            "quit" => {
                self.snapshot("final");
                for pair in &mut self.graphs {
                    if let Some((graph, capability)) = pair.take() {
                        graph
                            .retire(capability)
                            .unwrap_or_else(|e| panic!("retirement: {e}"));
                    }
                }
                assert_eq!(foxdaw_vst3_host::retained_graphs(), 0);
                self.retired = true;
                println!("clean_retirement retained=0");
            }
            other => panic!("unknown command {other}"),
        }
        self.status = command.to_string();
        println!("completed: {command}");
    }
}
impl eframe::App for Check {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        let binding = graphics_binding();
        for (graph, _) in self.graphs.iter_mut().flatten() {
            graph.pump_editor(Duration::ZERO);
            assert_eq!(
                graphics_binding(),
                binding,
                "pump preserves host GL binding"
            );
        }
        let commands = std::fs::read_to_string(&self.commands).unwrap_or_default();
        for line in commands.lines().skip(self.consumed) {
            if !line.trim().is_empty() {
                self.execute(line);
            }
            self.consumed += 1;
            assert_eq!(
                graphics_binding(),
                binding,
                "transition preserves host GL binding"
            );
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!(
                "{} — isolated native editor test",
                self.descriptor.name
            ));
            ui.label("Silent • disposable state • explicit rebuilt module");
            ui.label(&self.status);
        });
        if self.retired {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(15));
    }
}
fn main() -> eframe::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let descriptor = foxdaw_vst3_host::scan_module(&PathBuf::from(&args[0]))
        .unwrap()
        .remove(0);
    let mut check = Check {
        descriptor,
        graphs: vec![None, None],
        commands: PathBuf::from(&args[1]),
        consumed: 0,
        incarnation: 0,
        status: String::from("Ready"),
        retired: false,
    };
    let restore = args.get(2).map(|path| {
        serde_json::from_slice::<Vec<serde_json::Value>>(&std::fs::read(path).unwrap()).unwrap()
    });
    for index in 0..2 {
        let checkpoint = restore
            .as_ref()
            .map(|states| Checkpoint {
                component: serde_json::from_value::<Vec<u8>>(states[index]["component"].clone())
                    .unwrap()
                    .into(),
                controller: serde_json::from_value::<Vec<u8>>(states[index]["controller"].clone())
                    .unwrap()
                    .into(),
            })
            .unwrap_or_default();
        check.load(index, &checkpoint);
    }
    let result = eframe::run_native(
        "Foxplugs lifecycle check",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1100., 800.]),
            ..Default::default()
        },
        Box::new(move |cx| {
            assert!(cx.gl.is_some());
            Ok(Box::new(check))
        }),
    );
    println!("host_event_loop_returned={}", result.is_ok());
    // Ordinary process return also exercises plugin/module exit destructors.
    result
}

fn graphics_binding() -> [usize; 9] {
    unsafe {
        let egl = libloading::Library::new("libEGL.so.1").unwrap();
        let glx = libloading::Library::new("libGL.so.1").unwrap();
        type Get = unsafe extern "C" fn() -> *mut std::ffi::c_void;
        type GetSurface = unsafe extern "C" fn(u32) -> *mut std::ffi::c_void;
        type GetDrawable = unsafe extern "C" fn() -> libc::c_ulong;
        type GetApi = unsafe extern "C" fn() -> u32;
        [
            egl.get::<Get>(b"eglGetCurrentContext\0").unwrap()() as usize,
            egl.get::<Get>(b"eglGetCurrentDisplay\0").unwrap()() as usize,
            egl.get::<GetSurface>(b"eglGetCurrentSurface\0").unwrap()(0x3059) as usize,
            egl.get::<GetSurface>(b"eglGetCurrentSurface\0").unwrap()(0x305a) as usize,
            egl.get::<GetApi>(b"eglQueryAPI\0").unwrap()() as usize,
            glx.get::<Get>(b"glXGetCurrentContext\0").unwrap()() as usize,
            glx.get::<Get>(b"glXGetCurrentDisplay\0").unwrap()() as usize,
            glx.get::<GetDrawable>(b"glXGetCurrentDrawable\0").unwrap()() as usize,
            glx.get::<GetDrawable>(b"glXGetCurrentReadDrawable\0")
                .unwrap()() as usize,
        ]
    }
}
