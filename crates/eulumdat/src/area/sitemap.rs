//! A street map under the outdoor plan: OpenStreetMap tiles (the slippy map
//! layout of Web Mercator, 256 px per tile, the same tiles geodb-globe shows
//! with `?tiles=osm`) placed in the plan's metres around a site.
//!
//! The site (latitude, longitude) sits at a plan point (`anchor`, metres);
//! north is up: plan +x = east, plan +y = south. Over the few hundred metres
//! of a site the local plane is exact enough: metres per degree from the
//! Web Mercator sphere (radius 6 378 137 m), east–west scaled by the cosine
//! of the site's latitude.
//!
//! The tiles are loaded by the browser from the style's server
//! ([`STYLES`]: OpenStreetMap Standard from `tile.openstreetmap.org`, the
//! Humanitarian and CyclOSM styles hosted by OpenStreetMap France,
//! OpenTopoMap). Each asks for its attribution on the map (drawn by
//! [`SiteMap::svg`]), light use and no bulk downloads, e.g.
//! <https://operations.osmfoundation.org/policies/tiles/>.

use std::f64::consts::PI;

/// The OpenStreetMap tile URL, `{z}/{x}/{y}`.
pub const OSM_TILE_URL: &str = "https://tile.openstreetmap.org/{z}/{x}/{y}.png";
/// Deepest zoom the OpenStreetMap tile servers offer.
pub const OSM_MAX_ZOOM: u8 = 19;
/// Required on every map made from the tiles.
pub const OSM_ATTRIBUTION: &str = "© OpenStreetMap contributors";
pub const OSM_COPYRIGHT_URL: &str = "https://www.openstreetmap.org/copyright";

/// A tile style: where its tiles come from and what the map must credit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileStyle {
    /// Link key (`mapstyle=`).
    pub key: &'static str,
    /// Name shown in a picker (a proper name, not translated).
    pub label: &'static str,
    /// `{s}` (server a / b / c, when the host has them), `{z}`, `{x}`, `{y}`.
    pub url: &'static str,
    pub max_zoom: u8,
    pub attribution: &'static str,
    pub attribution_url: &'static str,
}

/// The tile styles a site map can use; the first is the default.
pub const STYLES: [TileStyle; 4] = [
    TileStyle {
        key: "osm",
        label: "OpenStreetMap Standard",
        url: OSM_TILE_URL,
        max_zoom: OSM_MAX_ZOOM,
        attribution: OSM_ATTRIBUTION,
        attribution_url: OSM_COPYRIGHT_URL,
    },
    TileStyle {
        key: "hot",
        label: "Humanitarian (HOT)",
        url: "https://{s}.tile.openstreetmap.fr/hot/{z}/{x}/{y}.png",
        max_zoom: 19,
        attribution: "© OpenStreetMap contributors · style: Humanitarian OpenStreetMap Team · hosted by OpenStreetMap France",
        attribution_url: OSM_COPYRIGHT_URL,
    },
    TileStyle {
        key: "cyclosm",
        label: "CyclOSM",
        url: "https://{s}.tile-cyclosm.openstreetmap.fr/cyclosm/{z}/{x}/{y}.png",
        max_zoom: 19,
        attribution: "© OpenStreetMap contributors · style: CyclOSM · hosted by OpenStreetMap France",
        attribution_url: "https://www.cyclosm.org",
    },
    TileStyle {
        key: "topo",
        label: "OpenTopoMap",
        url: "https://{s}.tile.opentopomap.org/{z}/{x}/{y}.png",
        max_zoom: 17,
        attribution: "© OpenStreetMap contributors, SRTM · style: © OpenTopoMap (CC-BY-SA)",
        attribution_url: "https://opentopomap.org/about",
    },
];

impl TileStyle {
    /// The style with this key, else OpenStreetMap Standard.
    pub fn by_key(key: &str) -> TileStyle {
        STYLES
            .iter()
            .copied()
            .find(|s| s.key == key)
            .unwrap_or(STYLES[0])
    }

    /// The URL of tile `z / x / y` (servers a / b / c in turn).
    pub fn url(&self, z: u8, x: u32, y: u32) -> String {
        let server = ["a", "b", "c"][((x + y) % 3) as usize];
        self.url
            .replace("{s}", server)
            .replace("{z}", &z.to_string())
            .replace("{x}", &x.to_string())
            .replace("{y}", &y.to_string())
    }
}

/// Web Mercator sphere radius (m).
const EARTH_RADIUS_M: f64 = 6_378_137.0;
/// Tile side (px).
const TILE_PX: f64 = 256.0;

/// One tile on the plan.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanTile {
    pub z: u8,
    pub x: u32,
    pub y: u32,
    /// The tile's corners on the plan (m): west, north, east, south.
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl PlanTile {
    pub fn url(&self) -> String {
        tile_url(self.z, self.x, self.y)
    }
}

/// The URL of tile `z / x / y`.
pub fn tile_url(z: u8, x: u32, y: u32) -> String {
    OSM_TILE_URL
        .replace("{z}", &z.to_string())
        .replace("{x}", &x.to_string())
        .replace("{y}", &y.to_string())
}

/// Tile column / row (fractional) of a point at zoom `z`.
pub fn tile_xy(lat: f64, lon: f64, z: u8) -> (f64, f64) {
    let n = f64::from(1u32 << z);
    let lat = lat.clamp(-85.051_128, 85.051_128).to_radians();
    let x = (lon + 180.0) / 360.0 * n;
    let y = (1.0 - lat.tan().asinh() / PI) / 2.0 * n;
    (x, y)
}

/// Latitude / longitude of tile coordinate (`x`, `y`) at zoom `z`.
pub fn tile_lat_lon(x: f64, y: f64, z: u8) -> (f64, f64) {
    let n = f64::from(1u32 << z);
    let lon = x / n * 360.0 - 180.0;
    let lat = (PI * (1.0 - 2.0 * y / n)).sinh().atan().to_degrees();
    (lat, lon)
}

/// The site and how its map is shown.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SiteMap {
    pub lat: f64,
    pub lon: f64,
    /// The plan point (m) at the site.
    pub anchor: (f64, f64),
    /// Opacity of the map (0–1).
    pub opacity: f64,
    /// Where the tiles come from ([`STYLES`]).
    pub style: TileStyle,
}

impl SiteMap {
    /// Metres per degree of latitude and of longitude at the site.
    pub fn metres_per_degree(&self) -> (f64, f64) {
        let lat = EARTH_RADIUS_M * PI / 180.0;
        (lat, lat * self.lat.to_radians().cos().max(1e-6))
    }

    /// Plan point (m) of a latitude / longitude.
    pub fn to_plan(&self, lat: f64, lon: f64) -> (f64, f64) {
        let (m_lat, m_lon) = self.metres_per_degree();
        (
            self.anchor.0 + (lon - self.lon) * m_lon,
            self.anchor.1 - (lat - self.lat) * m_lat,
        )
    }

    /// Latitude / longitude of a plan point (m).
    pub fn to_geo(&self, x: f64, y: f64) -> (f64, f64) {
        let (m_lat, m_lon) = self.metres_per_degree();
        (
            self.lat - (y - self.anchor.1) / m_lat,
            self.lon + (x - self.anchor.0) / m_lon,
        )
    }

    /// Ground size of one tile pixel at zoom `z` (m).
    pub fn metres_per_px(&self, z: u8) -> f64 {
        2.0 * PI * EARTH_RADIUS_M * self.lat.to_radians().cos() / (TILE_PX * f64::from(1u32 << z))
    }

    /// The zoom whose tile pixels are about as small as the plan's pixels
    /// (`px_per_m` on screen), at most the style's deepest zoom.
    pub fn zoom_for(&self, px_per_m: f64) -> u8 {
        let mut z = 1u8;
        while z < self.style.max_zoom && self.metres_per_px(z) > 1.0 / px_per_m.max(1e-6) {
            z += 1;
        }
        z
    }

    /// The tiles of zoom `z` that cover the plan rectangle (x0, y0) – (x1,
    /// y1) (m).
    pub fn tiles(&self, (x0, y0, x1, y1): (f64, f64, f64, f64), z: u8) -> Vec<PlanTile> {
        let (north, west) = self.to_geo(x0.min(x1), y0.min(y1));
        let (south, east) = self.to_geo(x0.max(x1), y0.max(y1));
        let (tx0, ty0) = tile_xy(north, west, z);
        let (tx1, ty1) = tile_xy(south, east, z);
        let n = 1i64 << z;
        let mut out = Vec::new();
        for ty in (ty0.floor() as i64).max(0)..=(ty1.floor() as i64).min(n - 1) {
            for tx in tx0.floor() as i64..=tx1.floor() as i64 {
                let (a_lat, a_lon) = tile_lat_lon(tx as f64, ty as f64, z);
                let (b_lat, b_lon) = tile_lat_lon(tx as f64 + 1.0, ty as f64 + 1.0, z);
                let (px0, py0) = self.to_plan(a_lat, a_lon);
                let (px1, py1) = self.to_plan(b_lat, b_lon);
                out.push(PlanTile {
                    z,
                    // wrap round the date line
                    x: tx.rem_euclid(n) as u32,
                    y: ty as u32,
                    x0: px0,
                    y0: py0,
                    x1: px1,
                    y1: py1,
                });
            }
        }
        out
    }

    /// The map as SVG for a plan drawn at `px = ox + x·sx`, `py = oy +
    /// y·sy`, covering the plan rectangle `bounds` (m), clipped to it, with
    /// the attribution in its lower right corner. The zoom follows the
    /// plan's scale (`screen_scale` = rendered px per SVG unit, 1 when
    /// unknown); at most `max_tiles` tiles (coarser zooms beyond).
    pub fn svg(
        &self,
        bounds: (f64, f64, f64, f64),
        (ox, oy, sx, sy): (f64, f64, f64, f64),
        screen_scale: f64,
        max_tiles: usize,
    ) -> String {
        let (x0, y0, x1, y1) = bounds;
        let px_per_m = sx.abs().max(sy.abs()) * screen_scale.max(0.1);
        let mut z = self.zoom_for(px_per_m);
        let mut tiles = self.tiles(bounds, z);
        while tiles.len() > max_tiles.max(1) && z > 1 {
            z -= 1;
            tiles = self.tiles(bounds, z);
        }
        let (cx0, cy0) = (ox + x0 * sx, oy + y0 * sy);
        let (cw, ch) = ((x1 - x0) * sx, (y1 - y0) * sy);
        let mut out = format!(
            r#"<g class="site-map" opacity="{:.2}"><clipPath id="siteMapClip"><rect x="{cx0:.1}" y="{cy0:.1}" width="{cw:.1}" height="{ch:.1}"/></clipPath><g clip-path="url(#siteMapClip)">"#,
            self.opacity.clamp(0.0, 1.0)
        );
        for t in &tiles {
            // a hair wider, so no seams show between tiles
            out.push_str(&format!(
                r#"<image href="{}" x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" preserveAspectRatio="none"/>"#,
                self.style.url(t.z, t.x, t.y),
                ox + t.x0 * sx,
                oy + t.y0 * sy,
                (t.x1 - t.x0) * sx + 0.6,
                (t.y1 - t.y0) * sy + 0.6
            ));
        }
        out.push_str("</g></g>");
        // the credit, as wide as its text (≈ 4.4 px per character at 8.5 px)
        let text = self.style.attribution;
        let width = (text.chars().count() as f64 * 4.4 + 8.0).min(cw.max(60.0));
        out.push_str(&format!(
            r##"<a href="{}" target="_blank"><rect x="{:.1}" y="{:.1}" width="{width:.1}" height="12" fill="#ffffff" fill-opacity="0.8"/><text x="{:.1}" y="{:.1}" font-size="8.5" fill="#333" text-anchor="end">{text}</text></a>"##,
            self.style.attribution_url,
            cx0 + cw - width,
            cy0 + ch - 12.0,
            cx0 + cw - 3.0,
            cy0 + ch - 3.0
        ));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site() -> SiteMap {
        // Lüdinghausen market place
        SiteMap {
            lat: 51.7686,
            lon: 7.4430,
            anchor: (25.0, 15.0),
            opacity: 0.8,
            style: STYLES[0],
        }
    }

    #[test]
    fn styles_bring_their_servers_zooms_and_credits() {
        assert_eq!(TileStyle::by_key("nope"), STYLES[0]);
        let hot = TileStyle::by_key("hot");
        assert_eq!(
            hot.url(18, 1, 1),
            "https://c.tile.openstreetmap.fr/hot/18/1/1.png"
        );
        assert_eq!(
            STYLES[0].url(16, 34122, 21715),
            "https://tile.openstreetmap.org/16/34122/21715.png"
        );
        // OpenTopoMap stops at zoom 17: a fine plan gets its deepest tiles
        let topo = SiteMap {
            style: TileStyle::by_key("topo"),
            ..site()
        };
        assert_eq!(topo.zoom_for(8.0), 17);
        let svg = topo.svg((0.0, 0.0, 50.0, 30.0), (40.0, 40.0, 8.4, 8.0), 1.0, 64);
        assert!(svg.contains("tile.opentopomap.org/17/") && svg.contains("OpenTopoMap (CC-BY-SA)"));
        // every style credits OpenStreetMap
        assert!(STYLES
            .iter()
            .all(|s| s.attribution.contains("OpenStreetMap contributors")));
        let mut keys: Vec<_> = STYLES.iter().map(|s| s.key).collect();
        keys.dedup();
        assert_eq!(keys.len(), STYLES.len());
    }

    #[test]
    fn plan_and_geo_round_trip() {
        let s = site();
        assert_eq!(s.to_plan(s.lat, s.lon), s.anchor);
        let (lat, lon) = s.to_geo(125.0, -35.0);
        let (x, y) = s.to_plan(lat, lon);
        assert!((x - 125.0).abs() < 1e-6 && (y + 35.0).abs() < 1e-6);
        // north is up: a point north of the site has a smaller y
        assert!(s.to_plan(s.lat + 0.001, s.lon).1 < s.anchor.1);
        // ~111 km per degree of latitude, ~69 km per degree of longitude here
        let (m_lat, m_lon) = s.metres_per_degree();
        assert!((m_lat - 111_319.5).abs() < 1.0);
        assert!((m_lon / m_lat - 51.7686f64.to_radians().cos()).abs() < 1e-12);
    }

    #[test]
    fn tile_maths_matches_the_slippy_map() {
        // z16 over Lüdinghausen (the standard slippy-map formula)
        let (x, y) = tile_xy(51.7686, 7.4430, 16);
        assert_eq!((x.floor(), y.floor()), (34122.0, 21715.0));
        // the equator and the prime meridian meet in the middle
        assert_eq!(tile_xy(0.0, 0.0, 1), (1.0, 1.0));
        let (lat, lon) = tile_lat_lon(x, y, 16);
        assert!((lat - 51.7686).abs() < 1e-9 && (lon - 7.4430).abs() < 1e-9);
        assert_eq!(
            tile_url(16, 34122, 21715),
            "https://tile.openstreetmap.org/16/34122/21715.png"
        );
    }

    #[test]
    fn tiles_cover_the_plan_and_are_square_on_the_ground() {
        let s = site();
        let z = 18;
        let tiles = s.tiles((0.0, 0.0, 50.0, 30.0), z);
        assert!(!tiles.is_empty() && tiles.len() <= 6, "{}", tiles.len());
        let west = tiles.iter().map(|t| t.x0).fold(f64::INFINITY, f64::min);
        let east = tiles.iter().map(|t| t.x1).fold(f64::NEG_INFINITY, f64::max);
        let north = tiles.iter().map(|t| t.y0).fold(f64::INFINITY, f64::min);
        let south = tiles.iter().map(|t| t.y1).fold(f64::NEG_INFINITY, f64::max);
        assert!(west <= 0.0 && east >= 50.0 && north <= 0.0 && south >= 30.0);
        // Web Mercator is conformal: a tile is square on the ground
        let t = &tiles[0];
        let (w, h) = (t.x1 - t.x0, t.y1 - t.y0);
        assert!((w / h - 1.0).abs() < 1e-3, "{w} × {h}");
        assert!((w - 256.0 * s.metres_per_px(z)).abs() / w < 1e-3);
    }

    #[test]
    fn the_zoom_follows_the_plan_and_the_svg_credits_osm() {
        let s = site();
        // 8 px per metre on screen: OSM's deepest zoom; 0.5 px/m: coarser
        assert_eq!(s.zoom_for(8.0), OSM_MAX_ZOOM);
        let coarse = s.zoom_for(0.5);
        assert!((14..=16).contains(&coarse), "{coarse}");
        let svg = s.svg((0.0, 0.0, 50.0, 30.0), (40.0, 40.0, 8.4, 8.0), 1.0, 64);
        assert!(svg.contains("tile.openstreetmap.org/19/"));
        assert!(svg.contains(OSM_ATTRIBUTION) && svg.contains(OSM_COPYRIGHT_URL));
        assert!(svg.contains(r#"opacity="0.80""#));
        // a large area at a fine scale: coarser zoom, within the tile cap
        let big = s.svg((0.0, 0.0, 2000.0, 2000.0), (40.0, 40.0, 8.0, 8.0), 1.0, 30);
        assert!(big.matches("<image").count() <= 30);
    }
}
