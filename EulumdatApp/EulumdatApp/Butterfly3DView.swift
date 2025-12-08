import SwiftUI
import SceneKit
import EulumdatKit

/// Real 3D photometric solid using SceneKit with Metal rendering
struct Butterfly3DView: View {
    let ldt: Eulumdat
    @State private var autoRotate = true
    @State private var showWireframe = false
    @State private var colorMode: ColorMode = .heatmap
    @Binding var isDarkTheme: Bool

    enum ColorMode: String, CaseIterable {
        case heatmap = "Heatmap"
        case cPlane = "C-Plane"
        case solid = "Solid"
    }

    var body: some View {
        VStack(spacing: 0) {
            SceneKitPhotometricView(
                ldt: ldt,
                autoRotate: $autoRotate,
                showWireframe: showWireframe,
                colorMode: colorMode,
                isDarkTheme: isDarkTheme
            )
            .overlay(alignment: .topLeading) {
                VStack(alignment: .leading, spacing: 2) {
                    Text("3D Photometric Solid")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Text("\(ldt.numCPlanes) C-planes × \(ldt.numGPlanes) γ-angles")
                        .font(.caption2)
                        .foregroundStyle(.tertiary)
                }
                .padding(8)
            }
            .overlay(alignment: .topTrailing) {
                Text("Drag to rotate • Scroll to zoom")
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
                    .padding(8)
            }
            .overlay(alignment: .bottomLeading) {
                VStack(alignment: .leading, spacing: 2) {
                    Text("Max: \(Int(ldt.maxIntensity)) cd/klm")
                    Text("Symmetry: \(symmetryName)")
                }
                .font(.caption2)
                .foregroundStyle(.secondary)
                .padding(8)
            }

            HStack(spacing: 12) {
                Button(autoRotate ? "Pause" : "Auto") {
                    autoRotate.toggle()
                }
                .buttonStyle(.bordered)

                Picker("Color", selection: $colorMode) {
                    ForEach(ColorMode.allCases, id: \.self) { mode in
                        Text(mode.rawValue).tag(mode)
                    }
                }
                .pickerStyle(.segmented)
                .frame(width: 200)

                Toggle("Wireframe", isOn: $showWireframe)
                    .toggleStyle(.button)
            }
            .padding(.vertical, 8)
        }
    }

    private var symmetryName: String {
        switch ldt.symmetry {
        case .none: return "None"
        case .verticalAxis: return "Vertical Axis"
        case .planeC0c180: return "C0-C180"
        case .planeC90c270: return "C90-C270"
        case .bothPlanes: return "Both Planes"
        }
    }
}

// MARK: - SceneKit View

// Universal SceneKit rendering logic
class SceneKitRenderer {
    let ldt: Eulumdat
    let colorMode: Butterfly3DView.ColorMode
    let isDarkTheme: Bool
    let showWireframe: Bool

    init(ldt: Eulumdat, colorMode: Butterfly3DView.ColorMode, isDarkTheme: Bool, showWireframe: Bool) {
        self.ldt = ldt
        self.colorMode = colorMode
        self.isDarkTheme = isDarkTheme
        self.showWireframe = showWireframe
    }

    func createScene() -> SCNScene {
        let scene = SCNScene()

        // Camera
        let cameraNode = SCNNode()
        cameraNode.camera = SCNCamera()
        cameraNode.camera?.zNear = 0.01
        cameraNode.camera?.zFar = 100
        cameraNode.position = SCNVector3(0, 1.2, 2.5)
        cameraNode.look(at: SCNVector3(0, 0, 0))
        scene.rootNode.addChildNode(cameraNode)

        // Lighting setup
        let ambientLight = SCNNode()
        ambientLight.light = SCNLight()
        ambientLight.light?.type = .ambient
        ambientLight.light?.intensity = 400
        #if os(macOS)
        ambientLight.light?.color = NSColor.white
        #else
        ambientLight.light?.color = UIColor.white
        #endif
        scene.rootNode.addChildNode(ambientLight)

        let keyLight = SCNNode()
        keyLight.light = SCNLight()
        keyLight.light?.type = .directional
        keyLight.light?.intensity = 800
        keyLight.light?.castsShadow = true
        keyLight.position = SCNVector3(3, 5, 3)
        keyLight.look(at: SCNVector3(0, 0, 0))
        scene.rootNode.addChildNode(keyLight)

        let fillLight = SCNNode()
        fillLight.light = SCNLight()
        fillLight.light?.type = .directional
        fillLight.light?.intensity = 300
        fillLight.position = SCNVector3(-3, 2, -1)
        fillLight.look(at: SCNVector3(0, 0, 0))
        scene.rootNode.addChildNode(fillLight)

        // Photometric solid
        let solidNode = SCNNode()
        solidNode.name = "photometricSolid"
        scene.rootNode.addChildNode(solidNode)

        buildPhotometricMesh(parent: solidNode)
        addGrid(parent: scene.rootNode)
        addAxisIndicators(parent: scene.rootNode)

        return scene
    }

    private func buildPhotometricMesh(parent: SCNNode) {
        guard !ldt.intensities.isEmpty, !ldt.gAngles.isEmpty else { return }

        let maxIntensity = max(ldt.maxIntensity, 1.0)
        let expandedCPlanes = expandCPlanes()

        guard expandedCPlanes.count >= 2 else {
            buildSimpleWings(parent: parent, cPlaneData: expandedCPlanes, maxIntensity: maxIntensity)
            return
        }

        let numC = expandedCPlanes.count
        let numG = ldt.gAngles.count

        var vertices: [SCNVector3] = []
        var normals: [SCNVector3] = []
        var colors: [CGFloat] = []
        var indices: [Int32] = []

        for (_, cData) in expandedCPlanes.enumerated() {
            let cAngle = cData.0
            let intensities = cData.1
            let cRad = Float(cAngle * .pi / 180.0)

            for (gIndex, gAngle) in ldt.gAngles.enumerated() {
                let intensity = gIndex < intensities.count ? intensities[gIndex] : 0
                let normalizedIntensity = intensity / maxIntensity
                let r = Float(normalizedIntensity)
                let gRad = Float(gAngle * .pi / 180.0)

                let x = r * sin(gRad) * cos(cRad)
                let z = r * sin(gRad) * sin(cRad)
                let y = -r * cos(gRad)

                vertices.append(SCNVector3(x, y, z))

                let len = sqrt(x*x + y*y + z*z)
                if len > 0.001 {
                    normals.append(SCNVector3(x/len, y/len, z/len))
                } else {
                    normals.append(SCNVector3(0, -1, 0))
                }

                let (cr, cg, cb, ca) = colorForVertex(intensity: normalizedIntensity, cAngle: cAngle, gAngle: gAngle)
                colors.append(contentsOf: [CGFloat(cr), CGFloat(cg), CGFloat(cb), CGFloat(ca)])
            }
        }

        for c in 0..<numC {
            let nextC = (c + 1) % numC
            for g in 0..<(numG - 1) {
                let v0 = Int32(c * numG + g)
                let v1 = Int32(nextC * numG + g)
                let v2 = Int32(nextC * numG + (g + 1))
                let v3 = Int32(c * numG + (g + 1))

                indices.append(contentsOf: [v0, v1, v2])
                indices.append(contentsOf: [v0, v2, v3])
            }
        }

        let vertexSource = SCNGeometrySource(vertices: vertices)
        let normalSource = SCNGeometrySource(normals: normals)

        let colorData = Data(bytes: colors, count: colors.count * MemoryLayout<CGFloat>.size)
        let colorSource = SCNGeometrySource(
            data: colorData,
            semantic: .color,
            vectorCount: vertices.count,
            usesFloatComponents: true,
            componentsPerVector: 4,
            bytesPerComponent: MemoryLayout<CGFloat>.size,
            dataOffset: 0,
            dataStride: MemoryLayout<CGFloat>.size * 4
        )

        let indexData = Data(bytes: indices, count: indices.count * MemoryLayout<Int32>.size)
        let element = SCNGeometryElement(
            data: indexData,
            primitiveType: .triangles,
            primitiveCount: indices.count / 3,
            bytesPerIndex: MemoryLayout<Int32>.size
        )

        let geometry = SCNGeometry(sources: [vertexSource, normalSource, colorSource], elements: [element])

        let material = SCNMaterial()
        material.isDoubleSided = true
        material.lightingModel = .physicallyBased
        material.metalness.contents = 0.0
        material.roughness.contents = 0.5
        material.fillMode = showWireframe ? .lines : .fill
        geometry.materials = [material]

        let meshNode = SCNNode(geometry: geometry)
        meshNode.name = "mesh"
        parent.addChildNode(meshNode)

        let centerSphere = SCNSphere(radius: 0.015)
        #if os(macOS)
        centerSphere.firstMaterial?.diffuse.contents = isDarkTheme ? NSColor.white : NSColor.darkGray
        #else
        centerSphere.firstMaterial?.diffuse.contents = isDarkTheme ? UIColor.white : UIColor.darkGray
        #endif
        centerSphere.firstMaterial?.lightingModel = .constant
        let centerNode = SCNNode(geometry: centerSphere)
        parent.addChildNode(centerNode)
    }

    private func colorForVertex(intensity: Double, cAngle: Double, gAngle: Double) -> (Float, Float, Float, Float) {
        switch colorMode {
        case .heatmap:
            return heatmapColor(intensity)
        case .cPlane:
            let hue = Float(cAngle / 360.0)
            let (r, g, b) = hslToRgb(h: hue, s: 0.7, l: 0.5)
            return (r, g, b, 0.9)
        case .solid:
            return (0.3, 0.5, 0.9, 0.85)
        }
    }

    private func heatmapColor(_ value: Double) -> (Float, Float, Float, Float) {
        let v = Float(max(0, min(1, value)))
        let r: Float, g: Float, b: Float

        if v < 0.25 {
            let t = v / 0.25
            r = 0; g = t; b = 1
        } else if v < 0.5 {
            let t = (v - 0.25) / 0.25
            r = 0; g = 1; b = 1 - t
        } else if v < 0.75 {
            let t = (v - 0.5) / 0.25
            r = t; g = 1; b = 0
        } else {
            let t = (v - 0.75) / 0.25
            r = 1; g = 1 - t; b = 0
        }

        return (r, g, b, 0.9)
    }

    private func hslToRgb(h: Float, s: Float, l: Float) -> (Float, Float, Float) {
        if s == 0 { return (l, l, l) }

        let q = l < 0.5 ? l * (1 + s) : l + s - l * s
        let p = 2 * l - q

        func hueToRgb(_ p: Float, _ q: Float, _ t: Float) -> Float {
            var t = t
            if t < 0 { t += 1 }
            if t > 1 { t -= 1 }
            if t < 1/6 { return p + (q - p) * 6 * t }
            if t < 1/2 { return q }
            if t < 2/3 { return p + (q - p) * (2/3 - t) * 6 }
            return p
        }

        return (
            hueToRgb(p, q, h + 1/3),
            hueToRgb(p, q, h),
            hueToRgb(p, q, h - 1/3)
        )
    }

    private func buildSimpleWings(parent: SCNNode, cPlaneData: [(Double, [Double])], maxIntensity: Double) {
        for (cAngle, intensities) in cPlaneData {
            let cRad = cAngle * .pi / 180.0
            var vertices: [SCNVector3] = [SCNVector3(0, 0, 0)]

            for (j, gAngle) in ldt.gAngles.enumerated() {
                let intensity = j < intensities.count ? intensities[j] : 0
                let r = Float(intensity / maxIntensity)
                let gRad = Float(gAngle) * .pi / 180.0

                let x = r * sin(gRad) * Float(cos(cRad))
                let y = -r * cos(gRad)
                let z = r * sin(gRad) * Float(sin(cRad))

                vertices.append(SCNVector3(x, y, z))
            }

            if vertices.count > 2 {
                var indices: [Int32] = []
                for i in 1..<(vertices.count - 1) {
                    indices.append(0)
                    indices.append(Int32(i))
                    indices.append(Int32(i + 1))
                }

                let vertexSource = SCNGeometrySource(vertices: vertices)
                let indexData = Data(bytes: indices, count: indices.count * MemoryLayout<Int32>.size)
                let element = SCNGeometryElement(
                    data: indexData,
                    primitiveType: .triangles,
                    primitiveCount: indices.count / 3,
                    bytesPerIndex: MemoryLayout<Int32>.size
                )

                let geometry = SCNGeometry(sources: [vertexSource], elements: [element])
                let hue = CGFloat(cAngle / 360.0)
                #if os(macOS)
                let color = NSColor(hue: hue, saturation: 0.6, brightness: 0.8, alpha: 0.7)
                #else
                let color = UIColor(hue: hue, saturation: 0.6, brightness: 0.8, alpha: 0.7)
                #endif

                let material = SCNMaterial()
                material.diffuse.contents = color
                material.isDoubleSided = true
                material.transparency = 0.7
                geometry.materials = [material]

                parent.addChildNode(SCNNode(geometry: geometry))
            }
        }
    }

    private func addGrid(parent: SCNNode) {
        #if os(macOS)
        let gridColor = isDarkTheme ? NSColor(white: 0.3, alpha: 1) : NSColor(white: 0.7, alpha: 1)
        #else
        let gridColor = isDarkTheme ? UIColor(white: 0.3, alpha: 1) : UIColor(white: 0.7, alpha: 1)
        #endif

        for i in 1...4 {
            let radius = CGFloat(i) / 4.0
            let ring = SCNTorus(ringRadius: radius, pipeRadius: 0.002)
            ring.firstMaterial?.diffuse.contents = gridColor
            ring.firstMaterial?.lightingModel = .constant
            let ringNode = SCNNode(geometry: ring)
            ringNode.eulerAngles.x = .pi / 2
            parent.addChildNode(ringNode)
        }

        let verticalPlane = SCNPlane(width: 2, height: 2)
        verticalPlane.firstMaterial?.diffuse.contents = gridColor.withAlphaComponent(0.1)
        verticalPlane.firstMaterial?.isDoubleSided = true
        verticalPlane.firstMaterial?.lightingModel = .constant
        let verticalPlaneNode = SCNNode(geometry: verticalPlane)
        verticalPlaneNode.eulerAngles.y = .pi / 2
        parent.addChildNode(verticalPlaneNode)
    }

    private func addAxisIndicators(parent: SCNNode) {
        let axisLength: Float = 1.2
        let axisRadius: CGFloat = 0.003

        // X axis (red)
        let xAxis = SCNCylinder(radius: axisRadius, height: CGFloat(axisLength))
        #if os(macOS)
        xAxis.firstMaterial?.diffuse.contents = NSColor.red
        #else
        xAxis.firstMaterial?.diffuse.contents = UIColor.red
        #endif
        xAxis.firstMaterial?.lightingModel = .constant
        let xNode = SCNNode(geometry: xAxis)
        xNode.position = SCNVector3(axisLength/2, 0, 0)
        xNode.eulerAngles.z = -.pi / 2
        parent.addChildNode(xNode)

        // Z axis (green)
        let zAxis = SCNCylinder(radius: axisRadius, height: CGFloat(axisLength))
        #if os(macOS)
        zAxis.firstMaterial?.diffuse.contents = NSColor.green
        #else
        zAxis.firstMaterial?.diffuse.contents = UIColor.green
        #endif
        zAxis.firstMaterial?.lightingModel = .constant
        let zNode = SCNNode(geometry: zAxis)
        zNode.position = SCNVector3(0, 0, axisLength/2)
        zNode.eulerAngles.x = .pi / 2
        parent.addChildNode(zNode)

        // Y axis (blue)
        let yAxis = SCNCylinder(radius: axisRadius, height: CGFloat(axisLength))
        #if os(macOS)
        yAxis.firstMaterial?.diffuse.contents = NSColor.blue
        #else
        yAxis.firstMaterial?.diffuse.contents = UIColor.blue
        #endif
        yAxis.firstMaterial?.lightingModel = .constant
        let yNode = SCNNode(geometry: yAxis)
        yNode.position = SCNVector3(0, -axisLength/2, 0)
        parent.addChildNode(yNode)

        addAxisLabel("C0", position: SCNVector3(axisLength + 0.05, 0, 0), parent: parent)
        addAxisLabel("C90", position: SCNVector3(0, 0, axisLength + 0.05), parent: parent)
        addAxisLabel("γ=0", position: SCNVector3(0, -axisLength - 0.05, 0), parent: parent)
    }

    private func addAxisLabel(_ text: String, position: SCNVector3, parent: SCNNode) {
        let textGeometry = SCNText(string: text, extrusionDepth: 0.01)
        #if os(macOS)
        textGeometry.font = NSFont.systemFont(ofSize: 0.08)
        textGeometry.firstMaterial?.diffuse.contents = isDarkTheme ? NSColor.white : NSColor.black
        #else
        textGeometry.font = UIFont.systemFont(ofSize: 0.08)
        textGeometry.firstMaterial?.diffuse.contents = isDarkTheme ? UIColor.white : UIColor.black
        #endif
        textGeometry.firstMaterial?.lightingModel = .constant

        let textNode = SCNNode(geometry: textGeometry)
        textNode.position = position
        textNode.scale = SCNVector3(0.5, 0.5, 0.5)

        let constraint = SCNBillboardConstraint()
        constraint.freeAxes = .all
        textNode.constraints = [constraint]

        parent.addChildNode(textNode)
    }

    private func expandCPlanes() -> [(Double, [Double])] {
        guard !ldt.intensities.isEmpty else { return [] }

        var result: [(Double, [Double])] = []

        switch ldt.symmetry {
        case .verticalAxis:
            let intensities = ldt.intensities[0]
            let numPlanes = 24
            for i in 0..<numPlanes {
                let cAngle = Double(i) * 360.0 / Double(numPlanes)
                result.append((cAngle, intensities))
            }

        case .planeC0c180:
            for (i, intensities) in ldt.intensities.enumerated() {
                guard i < ldt.cAngles.count else { continue }
                let cAngle = ldt.cAngles[i]
                result.append((cAngle, intensities))
                if cAngle > 0 && cAngle < 180 {
                    result.append((360.0 - cAngle, intensities))
                }
            }

        case .planeC90c270:
            for (i, intensities) in ldt.intensities.enumerated() {
                guard i < ldt.cAngles.count else { continue }
                let cAngle = ldt.cAngles[i]
                result.append((cAngle, intensities))
                if cAngle < 90 {
                    result.append((180.0 - cAngle, intensities))
                } else if cAngle > 90 && cAngle < 270 {
                    result.append((180.0 - cAngle + 360.0, intensities))
                }
            }

        case .bothPlanes:
            for (i, intensities) in ldt.intensities.enumerated() {
                guard i < ldt.cAngles.count else { continue }
                let cAngle = ldt.cAngles[i]
                result.append((cAngle, intensities))
                if cAngle > 0 && cAngle < 90 {
                    result.append((180.0 - cAngle, intensities))
                    result.append((180.0 + cAngle, intensities))
                    result.append((360.0 - cAngle, intensities))
                } else if abs(cAngle - 90.0) < 0.1 {
                    result.append((270.0, intensities))
                }
            }

        case .none:
            for (i, intensities) in ldt.intensities.enumerated() {
                guard i < ldt.cAngles.count else { continue }
                result.append((ldt.cAngles[i], intensities))
            }
        }

        result.sort { $0.0 < $1.0 }
        return result
    }
}

#if os(macOS)
struct SceneKitPhotometricView: NSViewRepresentable {
    let ldt: Eulumdat
    @Binding var autoRotate: Bool
    let showWireframe: Bool
    let colorMode: Butterfly3DView.ColorMode
    let isDarkTheme: Bool

    func makeCoordinator() -> Coordinator {
        Coordinator()
    }

    func makeNSView(context: Context) -> SCNView {
        let scnView = CustomSCNView()
        scnView.scene = createScene()
        scnView.allowsCameraControl = true
        scnView.autoenablesDefaultLighting = false
        scnView.backgroundColor = isDarkTheme ? NSColor(white: 0.1, alpha: 1) : NSColor.white
        scnView.antialiasingMode = .multisampling4X
        scnView.showsStatistics = false

        // Set up camera control with corrected mouse behavior
        scnView.defaultCameraController.interactionMode = .orbitTurntable
        scnView.defaultCameraController.inertiaEnabled = true

        return scnView
    }

    class Coordinator {
        // Empty coordinator for now
    }

    func updateNSView(_ scnView: SCNView, context: Context) {
        scnView.backgroundColor = isDarkTheme ? NSColor(white: 0.1, alpha: 1) : NSColor.white

        // Update auto-rotation
        if let solidNode = scnView.scene?.rootNode.childNode(withName: "photometricSolid", recursively: false) {
            if autoRotate {
                if solidNode.action(forKey: "autoRotate") == nil {
                    let rotation = SCNAction.rotateBy(x: 0, y: CGFloat.pi * 2, z: 0, duration: 20)
                    solidNode.runAction(SCNAction.repeatForever(rotation), forKey: "autoRotate")
                }
            } else {
                solidNode.removeAction(forKey: "autoRotate")
            }

            // Update wireframe mode
            if let meshNode = solidNode.childNode(withName: "mesh", recursively: false) {
                if let geometry = meshNode.geometry {
                    for material in geometry.materials {
                        material.fillMode = showWireframe ? .lines : .fill
                    }
                }
            }
        }
    }

    private func createScene() -> SCNScene {
        let renderer = SceneKitRenderer(ldt: ldt, colorMode: colorMode, isDarkTheme: isDarkTheme, showWireframe: showWireframe)
        let scene = renderer.createScene()

        // Start auto-rotation
        if autoRotate, let solidNode = scene.rootNode.childNode(withName: "photometricSolid", recursively: false) {
            let rotation = SCNAction.rotateBy(x: 0, y: CGFloat.pi * 2, z: 0, duration: 20)
            solidNode.runAction(SCNAction.repeatForever(rotation), forKey: "autoRotate")
        }

        return scene
    }
}

/// Custom SCNView that fixes mouse drag direction and adds Cmd+scroll zoom
class CustomSCNView: SCNView {
    override func scrollWheel(with event: NSEvent) {
        if event.modifierFlags.contains(.command) {
            // Cmd+scroll = zoom by moving camera closer/farther
            guard let cameraNode = pointOfView else {
                super.scrollWheel(with: event)
                return
            }

            // Get current camera position
            let position = cameraNode.position

            // Calculate zoom factor based on scroll delta
            let zoomSpeed: CGFloat = 0.1
            let delta = CGFloat(event.scrollingDeltaY) * zoomSpeed

            // Calculate direction from camera to origin
            let px = CGFloat(position.x)
            let py = CGFloat(position.y)
            let pz = CGFloat(position.z)
            let distance = sqrt(px * px + py * py + pz * pz)

            let dx = -px / distance
            let dy = -py / distance
            let dz = -pz / distance

            // Move camera along direction (zoom in/out)
            let newX = px + dx * delta
            let newY = py + dy * delta
            let newZ = pz + dz * delta

            let newPosition = SCNVector3(Float(newX), Float(newY), Float(newZ))

            // Clamp distance to prevent getting too close or too far
            let newDistance = sqrt(newX * newX + newY * newY + newZ * newZ)
            if newDistance > 0.5 && newDistance < 10.0 {
                cameraNode.position = newPosition
            }
        } else {
            // Normal scroll wheel behavior
            super.scrollWheel(with: event)
        }
    }

    override func mouseDragged(with event: NSEvent) {
        if event.modifierFlags.contains(.command) {
            // Cmd+drag = rotate camera around the scene
            guard let cameraNode = pointOfView else {
                super.mouseDragged(with: event)
                return
            }

            let sensitivity: CGFloat = 0.005

            // Invert the deltas for natural mouse movement
            let deltaX = CGFloat(-event.deltaX)
            let deltaY = CGFloat(-event.deltaY)

            // Get current orientation
            var eulerAngles = cameraNode.eulerAngles

            // Update rotation (Y for horizontal, X for vertical)
            eulerAngles.y += deltaX * sensitivity
            eulerAngles.x += deltaY * sensitivity

            // Clamp X rotation to prevent flipping
            let maxAngle: CGFloat = .pi / 2 - 0.1
            let minAngle: CGFloat = -.pi / 2 + 0.1
            eulerAngles.x = max(minAngle, min(maxAngle, eulerAngles.x))

            // Apply rotation
            cameraNode.eulerAngles = eulerAngles
        } else if allowsCameraControl {
            // Normal drag without Cmd = use built-in camera controls
            // Invert deltaX and deltaY to fix opposite mouse drag direction for mouse
            let cameraNode = pointOfView
            if let cameraNode = cameraNode {
                let sensitivity: CGFloat = 0.005

                // Invert the deltas for natural mouse movement
                let deltaX = CGFloat(-event.deltaX)
                let deltaY = CGFloat(-event.deltaY)

                // Get current orientation
                var eulerAngles = cameraNode.eulerAngles

                // Update rotation (Y for horizontal, X for vertical)
                eulerAngles.y += deltaX * sensitivity
                eulerAngles.x += deltaY * sensitivity

                // Clamp X rotation to prevent flipping
                let maxAngle: CGFloat = .pi / 2 - 0.1
                let minAngle: CGFloat = -.pi / 2 + 0.1
                eulerAngles.x = max(minAngle, min(maxAngle, eulerAngles.x))

                // Apply rotation
                cameraNode.eulerAngles = eulerAngles
            }
        } else {
            super.mouseDragged(with: event)
        }
    }
}
#endif

#if os(iOS)
struct SceneKitPhotometricView: UIViewRepresentable {
    let ldt: Eulumdat
    @Binding var autoRotate: Bool
    let showWireframe: Bool
    let colorMode: Butterfly3DView.ColorMode
    let isDarkTheme: Bool

    func makeUIView(context: Context) -> SCNView {
        let scnView = SCNView()
        scnView.scene = createScene()
        scnView.allowsCameraControl = true
        scnView.autoenablesDefaultLighting = false
        scnView.backgroundColor = isDarkTheme ? UIColor(white: 0.1, alpha: 1) : UIColor.white
        scnView.antialiasingMode = .multisampling4X
        return scnView
    }

    func updateUIView(_ scnView: SCNView, context: Context) {
        scnView.backgroundColor = isDarkTheme ? UIColor(white: 0.1, alpha: 1) : UIColor.white

        // Update auto-rotation
        if let solidNode = scnView.scene?.rootNode.childNode(withName: "photometricSolid", recursively: false) {
            if autoRotate {
                if solidNode.action(forKey: "autoRotate") == nil {
                    let rotation = SCNAction.rotateBy(x: 0, y: CGFloat.pi * 2, z: 0, duration: 20)
                    solidNode.runAction(SCNAction.repeatForever(rotation), forKey: "autoRotate")
                }
            } else {
                solidNode.removeAction(forKey: "autoRotate")
            }

            // Update wireframe mode
            if let meshNode = solidNode.childNode(withName: "mesh", recursively: false) {
                if let geometry = meshNode.geometry {
                    for material in geometry.materials {
                        material.fillMode = showWireframe ? .lines : .fill
                    }
                }
            }
        }
    }

    private func createScene() -> SCNScene {
        let renderer = SceneKitRenderer(ldt: ldt, colorMode: colorMode, isDarkTheme: isDarkTheme, showWireframe: showWireframe)
        let scene = renderer.createScene()

        // Start auto-rotation
        if autoRotate, let solidNode = scene.rootNode.childNode(withName: "photometricSolid", recursively: false) {
            let rotation = SCNAction.rotateBy(x: 0, y: CGFloat.pi * 2, z: 0, duration: 20)
            solidNode.runAction(SCNAction.repeatForever(rotation), forKey: "autoRotate")
        }

        return scene
    }
}
#endif

// MARK: - Standalone 3D Window View

struct Butterfly3DWindowView: View {
    @ObservedObject var model = Viewer3DModel.shared
    @AppStorage("isDarkTheme") private var isDarkTheme = false

    var body: some View {
        if let ldt = model.currentLDT {
            Butterfly3DView(ldt: ldt, isDarkTheme: $isDarkTheme)
                .navigationTitle("3D Photometric Solid - \(ldt.luminaireName)")
        } else {
            VStack(spacing: 16) {
                Image(systemName: "cube.transparent")
                    .font(.system(size: 80))
                    .foregroundStyle(.tertiary)
                Text("No data loaded")
                    .font(.title2)
                    .foregroundStyle(.secondary)
            }
            .frame(minWidth: 800, minHeight: 600)
        }
    }
}

// MARK: - Preview

#Preview {
    Butterfly3DView(
        ldt: Eulumdat(
            identification: "Test",
            typeIndicator: .pointSourceSymmetric,
            symmetry: .verticalAxis,
            numCPlanes: 1,
            cPlaneDistance: 0,
            numGPlanes: 19,
            gPlaneDistance: 5,
            measurementReportNumber: "",
            luminaireName: "Test",
            luminaireNumber: "",
            fileName: "",
            dateUser: "",
            length: 100,
            width: 100,
            height: 50,
            luminousAreaLength: 50,
            luminousAreaWidth: 50,
            heightC0: 0,
            heightC90: 0,
            heightC180: 0,
            heightC270: 0,
            downwardFluxFraction: 100,
            lightOutputRatio: 100,
            conversionFactor: 1,
            tiltAngle: 0,
            lampSets: [],
            directRatios: [],
            cAngles: [0],
            gAngles: Array(stride(from: 0.0, through: 90.0, by: 5.0)),
            intensities: [Array(repeating: 300.0, count: 19)],
            maxIntensity: 300,
            totalLuminousFlux: 1000
        ),
        isDarkTheme: .constant(false)
    )
}
