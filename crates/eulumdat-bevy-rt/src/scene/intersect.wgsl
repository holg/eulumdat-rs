#define_import_path eulumdat_rt::intersect

#import eulumdat_rt::common::{
    GpuPrimitive, HitRecord, PRIM_SHEET, EPSILON
}

fn intersect_sheet(
    ray_origin: vec3<f32>,
    ray_dir: vec3<f32>,
    prim: GpuPrimitive,
    t_min: f32,
    t_max: f32,
) -> HitRecord {
    var hit: HitRecord;
    hit.valid = false;

    let center = vec3<f32>(prim.params[0], prim.params[1], prim.params[2]);
    let normal = vec3<f32>(prim.params[3], prim.params[4], prim.params[5]);
    let u_axis = vec3<f32>(prim.params[6], prim.params[7], prim.params[8]);
    let half_w = prim.params[9];
    let half_h = prim.params[10];

    let denom = dot(ray_dir, normal);
    if (abs(denom) < EPSILON) { return hit; }

    let t = dot(center - ray_origin, normal) / denom;
    if (t < t_min || t > t_max) { return hit; }

    let p = ray_origin + t * ray_dir;
    let local = p - center;
    let v_axis = cross(normal, u_axis);
    let u = dot(local, u_axis);
    let v = dot(local, v_axis);

    if (abs(u) > half_w || abs(v) > half_h) { return hit; }

    hit.valid = true;
    hit.t = t;
    hit.point = p;
    hit.material_id = prim.material_id;
    hit.front_face = denom < 0.0;
    if (hit.front_face) { hit.normal = normal; }
    else { hit.normal = -normal; }

    return hit;
}

fn intersect_primitive(
    ray_origin: vec3<f32>,
    ray_dir: vec3<f32>,
    prim: GpuPrimitive,
    t_min: f32,
    t_max: f32,
) -> HitRecord {
    switch (prim.ptype) {
        case 0u: { // PRIM_SHEET
            return intersect_sheet(ray_origin, ray_dir, prim, t_min, t_max);
        }
        default: {
            var hit: HitRecord;
            hit.valid = false;
            return hit;
        }
    }
}
