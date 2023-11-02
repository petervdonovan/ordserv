use std::path::PathBuf;

const OUT_FILE_NAME: &str = "plots/throughput.png";

// use plotters::prelude::*;
use viz264::{get_atses, get_n_runs_over_time};

// 1.6G Nov  1 01:30 accumulating-traces-27428-367a86292fe265f952ae42931e8b3cb8.mpk
// 829M Nov  1 01:27 accumulating-traces-13440-367a86292fe265f952ae42931e8b3cb8.mpk
// 280M Nov  1 01:24 accumulating-traces-4521-367a86292fe265f952ae42931e8b3cb8.mpk
// 316K Nov  1 01:20 accumulating-traces-0-367a86292fe265f952ae42931e8b3cb8.mpk
// 315K Oct 30 22:07 known-counts-367a86292fe265f952ae42931e8b3cb8.mpk
// 5.8K Oct 30 17:36 compiled-367a86292fe265f952ae42931e8b3cb8.mpk

// 0 minutes: 0 runs
// 4 minutes: 4521 runs
// 7 minutes: 13440 runs
// 10 minutes: 27428 runs

// const OUT_FILE_NAME: &str = "plotters-doc-data/area-chart.png";
// fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let data: Vec<_> = {
//         let norm_dist = Normal::new(500.0, 100.0).unwrap();
//         let mut x_rand = XorShiftRng::from_seed(*b"MyFragileSeed123");
//         let x_iter = norm_dist.sample_iter(&mut x_rand);
//         x_iter
//             .filter(|x| *x < 1500.0)
//             .take(100)
//             .zip(0..)
//             .map(|(x, b)| x + (b as f64).powf(1.2))
//             .collect()
//     };

//     let root = BitMapBackend::new(OUT_FILE_NAME, (1024, 768)).into_drawing_area();

//     root.fill(&WHITE)?;

//     let mut chart = ChartBuilder::on(&root)
//         .set_label_area_size(LabelAreaPosition::Left, 60)
//         .set_label_area_size(LabelAreaPosition::Bottom, 60)
//         .caption("Area Chart Demo", ("sans-serif", 40))
//         .build_cartesian_2d(0..(data.len() - 1), 0.0..1500.0)?;

//     chart
//         .configure_mesh()
//         .disable_x_mesh()
//         .disable_y_mesh()
//         .draw()?;

//     chart.draw_series(
//         AreaSeries::new(
//             (0..).zip(data.iter()).map(|(x, y)| (x, *y)),
//             0.0,
//             RED.mix(0.2),
//         )
//         .border_style(RED),
//     )?;

//     // To avoid the IO failure being ignored silently, we manually call the present function
//     root.present().expect("Unable to write result to file, please make sure 'plotters-doc-data' dir exists under current dir");
//     println!("Result has been saved to {}", OUT_FILE_NAME);
//     Ok(())
// }

fn main() {
    let atses: Vec<protocol_test::testing::AccumulatingTracesState> =
        get_atses(&PathBuf::from("scratch"));
    let data = get_n_runs_over_time(&atses);
    println!("{:?}", data);
    // let root = BitMapBackend::new(OUT_FILE_NAME, (1024, 768)).into_drawing_area();
    // root.fill(&WHITE).expect("failed to fill white");
    // let mut chart = ChartBuilder::on(&root)
    //     .set_label_area_size(LabelAreaPosition::Left, 60)
    //     .set_label_area_size(LabelAreaPosition::Bottom, 60)
    //     .caption("Area Chart Demo", ("sans-serif", 40))
    //     .build_cartesian_2d(0..(data.len() - 1), 0.0..1500.0)
    //     .unwrap();

    // chart
    //     .configure_mesh()
    //     .disable_x_mesh()
    //     .disable_y_mesh()
    //     .draw()
    //     .unwrap();

    // chart
    //     .draw_series(
    //         plotters::series::LineSeries::new(
    //             data.iter()
    //                 .map(|(a, b)| (*a as usize, *b as f64))
    //                 .collect::<Vec<_>>(),
    //             0.0,
    //             RED.mix(0.2),
    //         )
    //         .border_style(RED),
    //     )
    //     .unwrap();
}
