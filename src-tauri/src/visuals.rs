use ort::session::Session;
use ort::value::Value;
use ort::inputs;
use image::{DynamicImage, imageops::FilterType};
use std::path::Path;
use anyhow::Result;

pub struct VisualEmbedder {
    session: Session,
}

impl VisualEmbedder {
    pub fn new(model_path: &Path) -> Result<Self> {
        let session = Session::builder()?
            .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .commit_from_file(model_path)?;
        
        Ok(Self { session })
    }

    pub fn generate_embedding(&self, img: DynamicImage) -> Result<Vec<f32>> {
        // CLIP expected input: 224x224 RGB, normalized
        let resized = img.resize_exact(224, 224, FilterType::Lanczos3).to_rgb8();
        
        // Normalize as per CLIP requirements
        let mut input_tensor = vec![0.0f32; 3 * 224 * 224];
        for (x, y, pixel) in resized.enumerate_pixels() {
            let r = (pixel[0] as f32 / 255.0 - 0.48145466) / 0.26862954;
            let g = (pixel[1] as f32 / 255.0 - 0.4578275) / 0.26130258;
            let b = (pixel[2] as f32 / 255.0 - 0.40821073) / 0.27577711;
            
            // Planar format (R, G, B)
            input_tensor[0 * 224 * 224 + y as usize * 224 + x as usize] = r;
            input_tensor[1 * 224 * 224 + y as usize * 224 + x as usize] = g;
            input_tensor[2 * 224 * 224 + y as usize * 224 + x as usize] = b;
        }

        let values = inputs![Value::from_array(([1, 3, 224, 224], input_tensor))? ]?;
        let outputs = self.session.run(values)?;
        
        let embedding: Vec<f32> = if let Some(output) = outputs.get("image_embeds") {
            let output_tensor = output.try_extract_tensor::<f32>()?;
            output_tensor.view().to_owned().into_iter().collect()
        } else if let Some(output) = outputs.values().next() {
            let output_tensor = output.try_extract_tensor::<f32>()?;
            output_tensor.view().to_owned().into_iter().collect()
        } else {
            return Err(anyhow::anyhow!("No output found"));
        };
        
        // L2 Normalize
        let norm = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        let normalized = embedding.into_iter().map(|x| x / norm).collect();
        
        Ok(normalized)
    }
}
