use std::{ path::PathBuf, time::Duration, process::ExitCode };

use bevy::{ app::{ AppExit, ScheduleRunnerPlugin }, log::{ Level, LogPlugin }, prelude::* };
use bevy_mod_environment_map_tools::write_ktx2;

use clap::Parser;

/// Encode Rgba16Float images as rgb9e5 in ktx2 files.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input file paths
    #[arg(short, long, value_delimiter = ',')]
    inputs: Vec<PathBuf>,

    /// Output file paths
    #[arg(short, long, value_delimiter = ',')]
    outputs: Vec<PathBuf>,
}

fn main() -> ExitCode {
    let args = Args::parse();

    if args.inputs.is_empty() {
        println!("No input paths provided");
        return ExitCode::FAILURE;
    }

    if args.outputs.is_empty() {
        println!("No output paths provided");
        return ExitCode::FAILURE;
    }

    if args.inputs.len() != args.outputs.len() {
        println!("Input and output path lengths don't match");
        return ExitCode::FAILURE;
    }

    let mut app = App::new();
    // TODO don't be ridiculous
    app.add_plugins(
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(1.0 / 100.0)))
            .build()
            .add(AssetPlugin::default())
            .add(ImagePlugin::default())
    ).add_systems(Update, convert);

    // Use bevy's logging for debug builds.
    #[cfg(debug_assertions)]
    {
        app.add_plugins(LogPlugin {
            level: Level::DEBUG,
            filter: "wgpu=error,bevy_render=info,bevy_ecs=trace".to_string(),
            update_subscriber: None,
        });
    }

    for (input, output) in args.inputs.iter().zip(args.outputs.iter()) {
        let asset_server = app.world.resource_mut::<AssetServer>();
        // using canonicalize to avoid being relative to the asset folder
        let image_h = asset_server.load(std::fs::canonicalize(input).unwrap());
        app.world.spawn(ImageToConvert {
            image_h,
            output_path: PathBuf::from(output),
        });
    }

    app.run();

    ExitCode::SUCCESS
}

#[derive(Component)]
struct Converted;

#[derive(Component)]
struct ImageToConvert {
    image_h: Handle<Image>,
    output_path: PathBuf,
}

fn convert(
    mut commands: Commands,
    query: Query<(Entity, &ImageToConvert), Without<Converted>>,
    images: ResMut<Assets<Image>>,
    mut app_exit_events: EventWriter<AppExit>
) {
    if query.is_empty() {
        app_exit_events.send(AppExit);
    }
    for (entity, conv) in &query {
        if let Some(image) = images.get(&conv.image_h) {
            println!(
                "Converting {}, {:?}, mip_level_count: {} format:{:?}",
                &conv.output_path.display(),
                image.texture_descriptor.size,
                image.texture_descriptor.mip_level_count,
                image.texture_descriptor.format
            );
            write_ktx2(image, &conv.output_path);
            commands.entity(entity).insert(Converted);
        }
    }
}
