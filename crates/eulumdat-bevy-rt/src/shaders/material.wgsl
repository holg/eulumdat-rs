#define_import_path eulumdat_rt::material

#import eulumdat_rt::common::{
    GpuMaterial, HitRecord, Interaction,
    MAT_ABSORBER, MAT_DIFFUSE_REFL, MAT_SPECULAR_REFL, MAT_CLEAR_TRANS, MAT_DIFFUSE_TRANS,
    INTERACTION_ABSORBED, INTERACTION_REFLECTED, INTERACTION_TRANSMITTED,
    EPSILON,
    reflect_dir, refract_dir, fresnel_schlick,
    random_f32, random_cosine_hemisphere, sample_henyey_greenstein,
}

fn interact_material(
    ray_dir: vec3<f32>,
    hit: HitRecord,
    mat: GpuMaterial,
) -> Interaction {
    var result: Interaction;
    result.attenuation = 1.0;

    switch (mat.mtype) {
        case 0u: { // MAT_ABSORBER
            result.itype = INTERACTION_ABSORBED;
            return result;
        }
        case 1u: { // MAT_DIFFUSE_REFL
            if (random_f32() > mat.reflectance) {
                result.itype = INTERACTION_ABSORBED;
                return result;
            }
            let new_dir = random_cosine_hemisphere(hit.normal);
            result.itype = INTERACTION_REFLECTED;
            result.new_origin = hit.point + new_dir * EPSILON;
            result.new_dir = new_dir;
            return result;
        }
        case 2u: { // MAT_SPECULAR_REFL
            if (random_f32() > mat.reflectance) {
                result.itype = INTERACTION_ABSORBED;
                return result;
            }
            let new_dir = reflect_dir(ray_dir, hit.normal);
            result.itype = INTERACTION_REFLECTED;
            result.new_origin = hit.point + new_dir * EPSILON;
            result.new_dir = normalize(new_dir);
            return result;
        }
        case 4u: { // MAT_CLEAR_TRANS
            var eta: f32;
            var cos_i: f32;
            if (hit.front_face) {
                eta = 1.0 / mat.ior;
                cos_i = clamp(-dot(ray_dir, hit.normal), 0.0, 1.0);
            } else {
                eta = mat.ior;
                cos_i = clamp(-dot(ray_dir, hit.normal), 0.0, 1.0);
            }
            let fr = max(fresnel_schlick(cos_i, eta), mat.min_reflectance);
            if (random_f32() < fr) {
                let refl = reflect_dir(ray_dir, hit.normal);
                result.itype = INTERACTION_REFLECTED;
                result.new_origin = hit.point + refl * EPSILON;
                result.new_dir = normalize(refl);
                return result;
            }
            let refr = refract_dir(ray_dir, hit.normal, eta);
            if (length(refr) < 0.5) { // TIR
                let refl = reflect_dir(ray_dir, hit.normal);
                result.itype = INTERACTION_REFLECTED;
                result.new_origin = hit.point + refl * EPSILON;
                result.new_dir = normalize(refl);
                return result;
            }
            let per_surface_tau = sqrt(mat.transmittance);
            result.itype = INTERACTION_TRANSMITTED;
            result.new_origin = hit.point + normalize(refr) * EPSILON;
            result.new_dir = normalize(refr);
            result.attenuation = per_surface_tau;
            return result;
        }
        case 5u: { // MAT_DIFFUSE_TRANS (thin-sheet model)
            var eta: f32;
            var cos_i: f32;
            if (hit.front_face) {
                eta = 1.0 / mat.ior;
                cos_i = clamp(-dot(ray_dir, hit.normal), 0.0, 1.0);
            } else {
                eta = mat.ior;
                cos_i = clamp(-dot(ray_dir, hit.normal), 0.0, 1.0);
            }
            let fr = max(fresnel_schlick(cos_i, eta), mat.min_reflectance);
            if (random_f32() < fr) {
                let refl = reflect_dir(ray_dir, hit.normal);
                result.itype = INTERACTION_REFLECTED;
                result.new_origin = hit.point + refl * EPSILON;
                result.new_dir = normalize(refl);
                return result;
            }
            let tau = exp(-mat.absorption_coeff * mat.thickness);
            if (random_f32() > tau) {
                result.itype = INTERACTION_ABSORBED;
                return result;
            }
            let refr = refract_dir(ray_dir, hit.normal, eta);
            if (length(refr) < 0.5) {
                result.itype = INTERACTION_ABSORBED;
                return result;
            }
            var exit_dir: vec3<f32>;
            if (mat.scattering_coeff > 0.0) {
                exit_dir = sample_henyey_greenstein(normalize(refr), mat.asymmetry);
            } else {
                exit_dir = normalize(refr);
            }
            var exit_eta: f32;
            if (hit.front_face) { exit_eta = mat.ior; }
            else { exit_eta = 1.0 / mat.ior; }
            let cos_exit = abs(dot(exit_dir, hit.normal));
            let exit_fr = max(fresnel_schlick(cos_exit, exit_eta), mat.min_reflectance);
            if (random_f32() < exit_fr) {
                result.itype = INTERACTION_ABSORBED;
                return result;
            }
            let exit_normal = select(hit.normal, -hit.normal, hit.front_face);
            let exit_refr = refract_dir(exit_dir, exit_normal, exit_eta);
            if (length(exit_refr) < 0.5) {
                result.itype = INTERACTION_ABSORBED;
                return result;
            }
            let exit_point = hit.point + exit_normal * mat.thickness + normalize(exit_refr) * EPSILON;
            result.itype = INTERACTION_TRANSMITTED;
            result.new_origin = exit_point;
            result.new_dir = normalize(exit_refr);
            return result;
        }
        default: {
            result.itype = INTERACTION_ABSORBED;
            return result;
        }
    }
}
