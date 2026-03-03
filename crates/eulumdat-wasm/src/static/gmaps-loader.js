// Lazy loader for Google Maps Lighting Designer
// Auto-generated - loads Google Maps API on demand
//
// The Maps API is NOT loaded until the user clicks "Maps Designer" tab.
// This keeps the initial page load fast.

// All variables prefixed with 'gmaps' to avoid conflicts with other loaders
let gmapsLoaded = false;
let gmapsLoading = false;
let gmapsLoadPromise = null;
let gmapsMap = null;
let gmapsDrawingManager = null;
let gmapsCurrentPolygon = null;
let gmapsLuminaires = [];
let gmapsHeatmapLayer = null;
let gmapsCalculationGrid = [];
let gmapsLuxLabels = [];
let gmapsShowLabels = true;

// Google Maps API key (injected at build time or from env)
const GMAPS_API_KEY = '__GMAPS_API_KEY__';

async function loadGoogleMaps() {
    if (gmapsLoaded) {
        console.log("[GMaps] Already loaded");
        return;
    }
    if (gmapsLoading && gmapsLoadPromise) {
        console.log("[GMaps] Loading in progress, waiting...");
        return gmapsLoadPromise;
    }

    gmapsLoading = true;
    console.log("[GMaps] Loading Google Maps API...");

    gmapsLoadPromise = new Promise((resolve, reject) => {
        // Create callback for Google Maps
        window.initGoogleMaps = () => {
            gmapsLoaded = true;
            gmapsLoading = false;
            console.log("[GMaps] Google Maps API loaded successfully");
            resolve();
        };

        // Load the script
        const script = document.createElement('script');
        script.src = `https://maps.googleapis.com/maps/api/js?key=${GMAPS_API_KEY}&libraries=drawing,visualization&callback=initGoogleMaps`;
        script.async = true;
        script.defer = true;
        script.onerror = (e) => {
            gmapsLoading = false;
            gmapsLoadPromise = null;
            console.error("[GMaps] Failed to load Google Maps API:", e);
            reject(new Error("Failed to load Google Maps API"));
        };
        document.head.appendChild(script);
    });

    return gmapsLoadPromise;
}

// Initialize the map in the given container
function initMap(containerId, options = {}) {
    if (!gmapsLoaded) {
        console.error("[GMaps] API not loaded yet");
        return null;
    }

    const container = document.getElementById(containerId);
    if (!container) {
        console.error("[GMaps] Container not found:", containerId);
        return null;
    }

    const defaultCenter = { lat: 48.8566, lng: 2.3522 }; // Paris
    const defaultOptions = {
        center: options.center || defaultCenter,
        zoom: options.zoom || 18,
        mapTypeId: google.maps.MapTypeId.SATELLITE,
        tilt: 0, // Top-down view for accurate measurements
        mapTypeControl: true,
        mapTypeControlOptions: {
            style: google.maps.MapTypeControlStyle.DROPDOWN_MENU,
            mapTypeIds: ['roadmap', 'satellite', 'hybrid']
        },
        streetViewControl: false,
        fullscreenControl: true,
    };

    gmapsMap = new google.maps.Map(container, defaultOptions);

    // Initialize drawing manager
    gmapsDrawingManager = new google.maps.drawing.DrawingManager({
        drawingMode: null,
        drawingControl: true,
        drawingControlOptions: {
            position: google.maps.ControlPosition.TOP_CENTER,
            drawingModes: [
                google.maps.drawing.OverlayType.POLYGON,
                google.maps.drawing.OverlayType.MARKER
            ]
        },
        polygonOptions: {
            fillColor: '#2196F3',
            fillOpacity: 0.2,
            strokeWeight: 2,
            strokeColor: '#1976D2',
            editable: true,
            draggable: true
        },
        markerOptions: {
            icon: {
                path: google.maps.SymbolPath.CIRCLE,
                scale: 10,
                fillColor: '#FFC107',
                fillOpacity: 1,
                strokeColor: '#FF9800',
                strokeWeight: 2
            },
            draggable: true
        }
    });

    gmapsDrawingManager.setMap(gmapsMap);

    // Handle polygon complete
    google.maps.event.addListener(gmapsDrawingManager, 'polygoncomplete', (polygon) => {
        if (gmapsCurrentPolygon) {
            gmapsCurrentPolygon.setMap(null);
        }
        gmapsCurrentPolygon = polygon;

        // Notify Leptos
        window.dispatchEvent(new CustomEvent('gmaps-polygon-complete', {
            detail: { polygon: getPolygonCoords(polygon) }
        }));

        // Recalculate when polygon is edited
        google.maps.event.addListener(polygon.getPath(), 'set_at', () => {
            window.dispatchEvent(new CustomEvent('gmaps-polygon-updated', {
                detail: { polygon: getPolygonCoords(polygon) }
            }));
        });
        google.maps.event.addListener(polygon.getPath(), 'insert_at', () => {
            window.dispatchEvent(new CustomEvent('gmaps-polygon-updated', {
                detail: { polygon: getPolygonCoords(polygon) }
            }));
        });
    });

    // Handle marker (luminaire) placement
    google.maps.event.addListener(gmapsDrawingManager, 'markercomplete', (marker) => {
        const luminaire = {
            id: Date.now(),
            marker: marker,
            position: marker.getPosition(),
            mountingHeight: 8.0,
            rotation: 0,
            tilt: 0
        };
        gmapsLuminaires.push(luminaire);

        // Add info window for luminaire settings
        const infoWindow = new google.maps.InfoWindow({
            content: createLuminaireInfoContent(luminaire)
        });

        marker.addListener('click', () => {
            infoWindow.open(gmapsMap, marker);
        });

        // Update position when dragged
        marker.addListener('dragend', () => {
            luminaire.position = marker.getPosition();
            window.dispatchEvent(new CustomEvent('gmaps-luminaire-moved', {
                detail: { luminaires: getLuminaireData() }
            }));
        });

        window.dispatchEvent(new CustomEvent('gmaps-luminaire-added', {
            detail: { luminaires: getLuminaireData() }
        }));
    });

    console.log("[GMaps] Map initialized");
    return gmapsMap;
}

// Get polygon coordinates as array of {lat, lng}
function getPolygonCoords(polygon) {
    const path = polygon.getPath();
    const coords = [];
    for (let i = 0; i < path.getLength(); i++) {
        const point = path.getAt(i);
        coords.push({ lat: point.lat(), lng: point.lng() });
    }
    return coords;
}

// Get luminaire data for calculations
function getLuminaireData() {
    return gmapsLuminaires.map(l => ({
        id: l.id,
        lat: l.position.lat(),
        lng: l.position.lng(),
        mountingHeight: l.mountingHeight,
        rotation: l.rotation,
        tilt: l.tilt
    }));
}

// Create info window content for luminaire settings
function createLuminaireInfoContent(luminaire) {
    return `
        <div style="padding: 8px; min-width: 200px;">
            <h4 style="margin: 0 0 8px 0;">Luminaire Settings</h4>
            <label style="display: block; margin: 4px 0;">
                Mounting Height (m):
                <input type="number" value="${luminaire.mountingHeight}" min="3" max="20" step="0.5"
                    onchange="window.updateLuminaire(${luminaire.id}, 'mountingHeight', this.value)"
                    style="width: 60px; margin-left: 8px;">
            </label>
            <label style="display: block; margin: 4px 0;">
                Rotation (°):
                <input type="number" value="${luminaire.rotation}" min="0" max="360" step="15"
                    onchange="window.updateLuminaire(${luminaire.id}, 'rotation', this.value)"
                    style="width: 60px; margin-left: 8px;">
            </label>
            <label style="display: block; margin: 4px 0;">
                Tilt (°):
                <input type="number" value="${luminaire.tilt}" min="0" max="45" step="5"
                    onchange="window.updateLuminaire(${luminaire.id}, 'tilt', this.value)"
                    style="width: 60px; margin-left: 8px;">
            </label>
            <button onclick="window.removeLuminaire(${luminaire.id})"
                style="margin-top: 8px; padding: 4px 8px; background: #f44336; color: white; border: none; border-radius: 4px; cursor: pointer;">
                Remove
            </button>
        </div>
    `;
}

// Update luminaire property
window.updateLuminaire = function(id, property, value) {
    const luminaire = gmapsLuminaires.find(l => l.id === id);
    if (luminaire) {
        luminaire[property] = parseFloat(value);
        window.dispatchEvent(new CustomEvent('gmaps-luminaire-updated', {
            detail: { luminaires: getLuminaireData() }
        }));
    }
};

// Remove luminaire
window.removeLuminaire = function(id) {
    const index = gmapsLuminaires.findIndex(l => l.id === id);
    if (index !== -1) {
        gmapsLuminaires[index].marker.setMap(null);
        gmapsLuminaires.splice(index, 1);
        window.dispatchEvent(new CustomEvent('gmaps-luminaire-removed', {
            detail: { luminaires: getLuminaireData() }
        }));
    }
};

// Calculate illuminance at a point from all luminaires
// ldtData: { intensities: [[f64]], c_angles: [f64], g_angles: [f64], lumens: f64 }
// point: { lat, lng }
// lumsData: [{ lat, lng, mountingHeight, rotation, tilt }]
function calculateIlluminanceAtPoint(ldtData, point, lumsData) {
    let totalLux = 0;

    for (const lum of lumsData) {
        // Calculate distance and angles
        const dx = (point.lng - lum.lng) * 111320 * Math.cos(lum.lat * Math.PI / 180); // meters
        const dy = (point.lat - lum.lat) * 110540; // meters
        const horizontalDist = Math.sqrt(dx * dx + dy * dy);
        const verticalDist = lum.mountingHeight;
        const totalDist = Math.sqrt(horizontalDist * horizontalDist + verticalDist * verticalDist);

        // Gamma angle (from vertical)
        const gamma = Math.atan2(horizontalDist, verticalDist) * 180 / Math.PI;

        // C angle (azimuth) - adjusted for luminaire rotation
        let c = Math.atan2(dx, dy) * 180 / Math.PI;
        c = (c - lum.rotation + 360) % 360;

        // Sample intensity from LDT data
        const intensity = sampleIntensity(ldtData, c, gamma);

        // Apply inverse square law with cosine correction
        // E = I * cos(gamma) / d^2
        const cosGamma = Math.cos(gamma * Math.PI / 180);
        const lux = (intensity * cosGamma) / (totalDist * totalDist);

        totalLux += lux;
    }

    return totalLux;
}

// Sample intensity from LDT data at given C and gamma angles
function sampleIntensity(ldtData, c, gamma) {
    if (!ldtData || !ldtData.intensities || ldtData.intensities.length === 0) {
        return 0;
    }

    const cAngles = ldtData.c_angles || [0];
    const gAngles = ldtData.g_angles || [];

    if (gAngles.length === 0) return 0;

    // Find nearest C-plane indices
    let cIdx0 = 0, cIdx1 = 0;
    for (let i = 0; i < cAngles.length - 1; i++) {
        if (c >= cAngles[i] && c <= cAngles[i + 1]) {
            cIdx0 = i;
            cIdx1 = i + 1;
            break;
        }
    }

    // Find nearest gamma indices
    let gIdx0 = 0, gIdx1 = 0;
    for (let i = 0; i < gAngles.length - 1; i++) {
        if (gamma >= gAngles[i] && gamma <= gAngles[i + 1]) {
            gIdx0 = i;
            gIdx1 = i + 1;
            break;
        }
    }

    // Bilinear interpolation
    const cRange = cAngles[cIdx1] - cAngles[cIdx0] || 1;
    const gRange = gAngles[gIdx1] - gAngles[gIdx0] || 1;
    const cFrac = (c - cAngles[cIdx0]) / cRange;
    const gFrac = (gamma - gAngles[gIdx0]) / gRange;

    const i00 = ldtData.intensities[cIdx0]?.[gIdx0] || 0;
    const i01 = ldtData.intensities[cIdx0]?.[gIdx1] || 0;
    const i10 = ldtData.intensities[cIdx1]?.[gIdx0] || 0;
    const i11 = ldtData.intensities[cIdx1]?.[gIdx1] || 0;

    const i0 = i00 * (1 - gFrac) + i01 * gFrac;
    const i1 = i10 * (1 - gFrac) + i11 * gFrac;
    const intensity = i0 * (1 - cFrac) + i1 * cFrac;

    // Convert from cd/klm to cd using lumens
    const lumens = ldtData.lumens || 1000;
    return intensity * (lumens / 1000);
}

// Calculate and display heatmap
function calculateHeatmap(ldtData, gridSpacing = 1.0) {
    if (!gmapsCurrentPolygon || gmapsLuminaires.length === 0) {
        console.log("[GMaps] No polygon or luminaires to calculate");
        return null;
    }

    const bounds = new google.maps.LatLngBounds();
    const path = gmapsCurrentPolygon.getPath();
    for (let i = 0; i < path.getLength(); i++) {
        bounds.extend(path.getAt(i));
    }

    const ne = bounds.getNorthEast();
    const sw = bounds.getSouthWest();

    // Calculate grid
    const latStep = gridSpacing / 110540; // degrees per meter
    const lngStep = gridSpacing / (111320 * Math.cos(((ne.lat() + sw.lat()) / 2) * Math.PI / 180));

    gmapsCalculationGrid = [];
    const heatmapData = [];
    let minLux = Infinity, maxLux = 0, sumLux = 0, count = 0;

    for (let lat = sw.lat(); lat <= ne.lat(); lat += latStep) {
        for (let lng = sw.lng(); lng <= ne.lng(); lng += lngStep) {
            const point = new google.maps.LatLng(lat, lng);

            // Check if point is inside polygon
            if (google.maps.geometry.poly.containsLocation(point, gmapsCurrentPolygon)) {
                const lux = calculateIlluminanceAtPoint(ldtData, { lat, lng }, getLuminaireData());

                gmapsCalculationGrid.push({ lat, lng, lux });
                heatmapData.push({
                    location: point,
                    weight: lux
                });

                minLux = Math.min(minLux, lux);
                maxLux = Math.max(maxLux, lux);
                sumLux += lux;
                count++;
            }
        }
    }

    // Remove old heatmap and labels
    if (gmapsHeatmapLayer) {
        gmapsHeatmapLayer.setMap(null);
    }
    gmapsLuxLabels.forEach(label => label.setMap(null));
    gmapsLuxLabels = [];

    // Create new heatmap
    gmapsHeatmapLayer = new google.maps.visualization.HeatmapLayer({
        data: heatmapData,
        map: gmapsMap,
        radius: 20,
        opacity: 0.7,
        gradient: [
            'rgba(0, 0, 0, 0)',      // 0
            'rgba(0, 0, 139, 0.8)',   // dark blue
            'rgba(0, 0, 255, 0.8)',   // blue
            'rgba(0, 255, 255, 0.8)', // cyan
            'rgba(0, 255, 0, 0.8)',   // green
            'rgba(255, 255, 0, 0.8)', // yellow
            'rgba(255, 128, 0, 0.8)', // orange
            'rgba(255, 0, 0, 0.8)'    // red
        ]
    });

    const avgLux = count > 0 ? sumLux / count : 0;
    const uniformity = count > 0 ? minLux / avgLux : 0;

    // Add lux value labels at sparse intervals (every 5th point or ~5m)
    // This prevents cluttering the map with too many labels
    if (gmapsShowLabels) {
        const labelInterval = Math.max(1, Math.floor(5 / gridSpacing)); // Show label every ~5m
        let pointIndex = 0;

        // Calculate grid dimensions for proper row/column skipping
        const gridWidth = Math.ceil((ne.lng() - sw.lng()) / lngStep);

        for (const point of gmapsCalculationGrid) {
            const row = Math.floor(pointIndex / gridWidth);
            const col = pointIndex % gridWidth;

            // Only show labels at sparse intervals
            if (row % labelInterval === 0 && col % labelInterval === 0) {
                const label = new google.maps.Marker({
                    position: { lat: point.lat, lng: point.lng },
                    map: gmapsMap,
                    icon: {
                        path: google.maps.SymbolPath.CIRCLE,
                        scale: 0  // Invisible marker, just for the label
                    },
                    label: {
                        text: point.lux.toFixed(0),
                        color: '#ffffff',
                        fontSize: '11px',
                        fontWeight: 'bold'
                    }
                });
                gmapsLuxLabels.push(label);
            }
            pointIndex++;
        }
        console.log(`[GMaps] Added ${gmapsLuxLabels.length} lux labels (every ${labelInterval * gridSpacing}m)`);
    }

    console.log(`[GMaps] Heatmap calculated: ${count} points, min=${minLux.toFixed(1)} lux, max=${maxLux.toFixed(1)} lux, avg=${avgLux.toFixed(1)} lux, U0=${uniformity.toFixed(2)}`);

    return {
        minLux,
        maxLux,
        avgLux,
        uniformity,
        pointCount: count,
        grid: gmapsCalculationGrid
    };
}

// Clear all drawings
function clearAll() {
    if (gmapsCurrentPolygon) {
        gmapsCurrentPolygon.setMap(null);
        gmapsCurrentPolygon = null;
    }
    gmapsLuminaires.forEach(l => l.marker.setMap(null));
    gmapsLuminaires = [];
    if (gmapsHeatmapLayer) {
        gmapsHeatmapLayer.setMap(null);
        gmapsHeatmapLayer = null;
    }
    // Clear lux labels
    gmapsLuxLabels.forEach(label => label.setMap(null));
    gmapsLuxLabels = [];
    gmapsCalculationGrid = [];
}

// Toggle lux labels visibility
function toggleLuxLabels(show) {
    gmapsShowLabels = show;
    gmapsLuxLabels.forEach(label => label.setMap(show ? gmapsMap : null));
}

// Export to CSV
function exportToCsv() {
    if (gmapsCalculationGrid.length === 0) {
        return null;
    }

    let csv = 'Latitude,Longitude,Illuminance (lux)\n';
    for (const point of gmapsCalculationGrid) {
        csv += `${point.lat.toFixed(8)},${point.lng.toFixed(8)},${point.lux.toFixed(2)}\n`;
    }
    return csv;
}

// Get user's location
function centerOnUserLocation() {
    if (navigator.geolocation) {
        navigator.geolocation.getCurrentPosition(
            (position) => {
                const pos = {
                    lat: position.coords.latitude,
                    lng: position.coords.longitude
                };
                gmapsMap.setCenter(pos);
                gmapsMap.setZoom(18);
            },
            () => {
                console.log("[GMaps] Geolocation failed");
            }
        );
    }
}

// Expose functions to window for Leptos interop
window.loadGoogleMaps = loadGoogleMaps;
window.initGMapsDesigner = initMap;
window.calculateGMapsHeatmap = calculateHeatmap;
window.clearGMapsDesigner = clearAll;
window.exportGMapsToCsv = exportToCsv;
window.centerOnUserLocation = centerOnUserLocation;
window.toggleGMapsLuxLabels = toggleLuxLabels;
window.isGMapsLoaded = () => gmapsLoaded;
window.isGMapsLoading = () => gmapsLoading;
window.getGMapsLuminaires = getLuminaireData;
window.getGMapsPolygon = () => gmapsCurrentPolygon ? getPolygonCoords(gmapsCurrentPolygon) : null;

console.log("[GMaps] Loader ready");
