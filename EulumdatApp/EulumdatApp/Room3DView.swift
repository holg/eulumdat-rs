import SwiftUI
import SceneKit
import EulumdatKit

/// Scene type presets for different luminaire applications
enum SceneType: String, CaseIterable, Identifiable {
    case room = "Room"
    case road = "Road"
    case parking = "Parking"
    case outdoor = "Garden"

    var id: String { rawValue }

    var icon: String {
        switch self {
        case .room: return "house.fill"
        case .road: return "road.lanes"
        case .parking: return "parkingsign"
        case .outdoor: return "tree.fill"
        }
    }

    var description: String {
        switch self {
        case .room: return "Indoor room"
        case .road: return "Street/Road"
        case .parking: return "Parking lot"
        case .outdoor: return "Garden/Park"
        }
    }

    /// Default dimensions for this scene type
    var defaultDimensions: (width: Double, length: Double, height: Double, mountHeight: Double) {
        switch self {
        case .room: return (4.0, 5.0, 2.8, 2.5)
        case .road: return (10.0, 30.0, 0.0, 8.0)      // Road: wide, long, pole height
        case .parking: return (20.0, 30.0, 0.0, 6.0)   // Parking: large area
        case .outdoor: return (10.0, 15.0, 0.0, 3.0)   // Garden: smaller pole
        }
    }

    /// Camera starting position for this scene
    var cameraPosition: (x: Float, y: Float, z: Float) {
        switch self {
        case .room: return (0.5, 1.6, 0.5)             // Inside room, eye level
        case .road: return (5.0, 1.7, 2.0)             // Standing on sidewalk
        case .parking: return (10.0, 1.7, 5.0)         // In the parking lot
        case .outdoor: return (5.0, 1.7, 3.0)          // In the garden
        }
    }
}

/// 3D Scene view with IES lighting from the loaded luminaire
struct Room3DView: View {
    let ldt: Eulumdat
    @State private var sceneType: SceneType = .room
    @State private var roomWidth: Double = 4.0    // meters
    @State private var roomLength: Double = 5.0   // meters
    @State private var roomHeight: Double = 2.8   // meters (0 for outdoor)
    @State private var mountingHeight: Double = 2.5 // meters from floor/ground
    @State private var lightIntensity: Double = 1000 // lumens (will be updated from lamp data)
    @State private var showLuminaire: Bool = true
    @State private var showPhotometricSolid: Bool = false
    @State private var wallColor: WallColor = .white
    @State private var hasInitializedIntensity: Bool = false
    @Binding var isDarkTheme: Bool

    /// Extract color temperature in Kelvin from lamp's colorAppearance string
    /// Handles formats like "3000K", "3000", "3000 K", "warm white (3000K)"
    private var colorTemperature: Double {
        guard let firstLamp = ldt.lampSets.first else { return 4000 } // Default daylight
        let appearance = firstLamp.colorAppearance

        // Extract numeric value (supports "3000K", "3000", "3000 K", etc.)
        let pattern = #"(\d{4})"# // Look for 4-digit number (typical CCT range 1800-10000)
        if let regex = try? NSRegularExpression(pattern: pattern),
           let match = regex.firstMatch(in: appearance, range: NSRange(appearance.startIndex..., in: appearance)),
           let range = Range(match.range(at: 1), in: appearance) {
            return Double(appearance[range]) ?? 4000
        }

        // Fallback: try parsing the whole string as a number
        let numericPart = appearance.replacingOccurrences(of: "[^0-9]", with: "", options: .regularExpression)
        if let kelvin = Double(numericPart), kelvin >= 1000, kelvin <= 20000 {
            return kelvin
        }

        return 4000 // Default neutral white
    }

    /// Get CRI value from lamp's colorRenderingGroup
    private var colorRenderingIndex: String {
        guard let firstLamp = ldt.lampSets.first else { return "N/A" }
        let cri = firstLamp.colorRenderingGroup
        return cri.isEmpty ? "N/A" : cri
    }

    /// Get total luminous flux from lamp sets
    private var totalLampFlux: Double {
        let flux = ldt.lampSets.reduce(0.0) { $0 + $1.totalLuminousFlux }
        return flux > 0 ? flux : ldt.totalLuminousFlux // Fallback to file-level flux
    }

    enum WallColor: String, CaseIterable {
        case white = "White"
        case cream = "Cream"
        case gray = "Gray"
        case wood = "Wood"

        var color: (wall: Any, floor: Any) {
            #if os(macOS)
            switch self {
            case .white: return (NSColor(white: 0.95, alpha: 1), NSColor(white: 0.85, alpha: 1))
            case .cream: return (NSColor(red: 0.96, green: 0.94, blue: 0.88, alpha: 1), NSColor(red: 0.8, green: 0.7, blue: 0.5, alpha: 1))
            case .gray: return (NSColor(white: 0.7, alpha: 1), NSColor(white: 0.5, alpha: 1))
            case .wood: return (NSColor(red: 0.9, green: 0.85, blue: 0.75, alpha: 1), NSColor(red: 0.6, green: 0.4, blue: 0.2, alpha: 1))
            }
            #else
            switch self {
            case .white: return (UIColor(white: 0.95, alpha: 1), UIColor(white: 0.85, alpha: 1))
            case .cream: return (UIColor(red: 0.96, green: 0.94, blue: 0.88, alpha: 1), UIColor(red: 0.8, green: 0.7, blue: 0.5, alpha: 1))
            case .gray: return (UIColor(white: 0.7, alpha: 1), UIColor(white: 0.5, alpha: 1))
            case .wood: return (UIColor(red: 0.9, green: 0.85, blue: 0.75, alpha: 1), UIColor(red: 0.6, green: 0.4, blue: 0.2, alpha: 1))
            }
            #endif
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            // 3D Scene with SceneKit's built-in camera controls
            RoomSceneView(
                ldt: ldt,
                sceneType: sceneType,
                roomWidth: roomWidth,
                roomLength: roomLength,
                roomHeight: roomHeight,
                mountingHeight: mountingHeight,
                lightIntensity: lightIntensity,
                colorTemperature: colorTemperature,
                showLuminaire: showLuminaire,
                showPhotometricSolid: showPhotometricSolid,
                wallColor: wallColor,
                isDarkTheme: isDarkTheme
            )
            .onAppear {
                // Initialize intensity from lamp data once
                if !hasInitializedIntensity && totalLampFlux > 0 {
                    lightIntensity = totalLampFlux
                    hasInitializedIntensity = true
                }
            }
            .onChange(of: sceneType) { newType in
                // Apply default dimensions for scene type
                let dims = newType.defaultDimensions
                roomWidth = dims.width
                roomLength = dims.length
                roomHeight = dims.height
                mountingHeight = dims.mountHeight
            }
            .overlay(alignment: .topLeading) {
                VStack(alignment: .leading, spacing: 2) {
                    Text("\(sceneType.description) Simulation")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    if sceneType == .room {
                        Text("\(String(format: "%.1f", roomWidth))×\(String(format: "%.1f", roomLength))×\(String(format: "%.1f", roomHeight))m")
                            .font(.caption2)
                            .foregroundStyle(.tertiary)
                    } else {
                        Text("\(String(format: "%.0f", roomWidth))×\(String(format: "%.0f", roomLength))m • \(String(format: "%.1f", mountingHeight))m pole")
                            .font(.caption2)
                            .foregroundStyle(.tertiary)
                    }
                    // Show lamp color info
                    HStack(spacing: 8) {
                        Text("\(Int(colorTemperature))K")
                            .font(.caption2)
                            .foregroundStyle(colorTemperature < 3500 ? .orange : (colorTemperature > 5000 ? .cyan : .yellow))
                        if colorRenderingIndex != "N/A" {
                            Text("CRI: \(colorRenderingIndex)")
                                .font(.caption2)
                                .foregroundStyle(.tertiary)
                        }
                    }
                }
                .padding(8)
                .background(.ultraThinMaterial)
                .clipShape(RoundedRectangle(cornerRadius: 8))
                .padding(8)
            }
            .overlay(alignment: .topTrailing) {
                // Navigation help
                VStack(alignment: .trailing, spacing: 2) {
                    #if os(macOS)
                    Text("Drag: Look around")
                    Text("WASD/Arrows: Walk")
                    Text("Q/E: Up/Down")
                    Text("Double-click: Teleport")
                    #else
                    Text("Drag: Look around")
                    Text("Pinch: Walk")
                    #endif
                }
                .font(.caption2)
                .foregroundStyle(.tertiary)
                .padding(8)
                .background(.ultraThinMaterial)
                .clipShape(RoundedRectangle(cornerRadius: 8))
                .padding(8)
            }

            Divider()

            // Controls
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 16) {
                    // Scene type picker
                    VStack(alignment: .leading, spacing: 4) {
                        Text("Scene").font(.caption).foregroundStyle(.secondary)
                        Picker("", selection: $sceneType) {
                            ForEach(SceneType.allCases) { type in
                                Label(type.rawValue, systemImage: type.icon).tag(type)
                            }
                        }
                        .pickerStyle(.segmented)
                        .frame(width: 200)
                    }

                    Divider().frame(height: 50)

                    // Dimensions
                    VStack(alignment: .leading, spacing: 4) {
                        Text(sceneType == .room ? "Room Size" : "Area Size").font(.caption).foregroundStyle(.secondary)
                        HStack(spacing: 8) {
                            VStack(spacing: 2) {
                                Text("W").font(.caption2).foregroundStyle(.tertiary)
                                Stepper(value: $roomWidth, in: 2...50, step: sceneType == .room ? 0.5 : 2.0) {
                                    Text("\(String(format: sceneType == .room ? "%.1f" : "%.0f", roomWidth))m")
                                        .font(.caption)
                                        .monospacedDigit()
                                        .frame(width: 40)
                                }
                                .labelsHidden()
                            }
                            VStack(spacing: 2) {
                                Text("L").font(.caption2).foregroundStyle(.tertiary)
                                Stepper(value: $roomLength, in: 2...100, step: sceneType == .room ? 0.5 : 5.0) {
                                    Text("\(String(format: sceneType == .room ? "%.1f" : "%.0f", roomLength))m")
                                        .font(.caption)
                                        .monospacedDigit()
                                        .frame(width: 40)
                                }
                                .labelsHidden()
                            }
                            // Room height only for indoor scenes
                            if sceneType == .room {
                                VStack(spacing: 2) {
                                    Text("H").font(.caption2).foregroundStyle(.tertiary)
                                    Stepper(value: $roomHeight, in: 2...5, step: 0.1) {
                                        Text("\(String(format: "%.1f", roomHeight))m")
                                            .font(.caption)
                                            .monospacedDigit()
                                            .frame(width: 40)
                                    }
                                    .labelsHidden()
                                }
                            }
                        }
                    }

                    Divider().frame(height: 50)

                    // Mounting/Pole height
                    VStack(alignment: .leading, spacing: 4) {
                        Text(sceneType == .room ? "Mount Height" : "Pole Height").font(.caption).foregroundStyle(.secondary)
                        Stepper(value: $mountingHeight, in: sceneType == .room ? 1.5...max(1.6, roomHeight - 0.1) : 2...15, step: sceneType == .room ? 0.1 : 0.5) {
                            Text("\(String(format: "%.1f", mountingHeight))m")
                                .font(.caption)
                                .monospacedDigit()
                        }
                    }

                    Divider().frame(height: 50)

                    // Light intensity
                    VStack(alignment: .leading, spacing: 4) {
                        Text("Intensity").font(.caption).foregroundStyle(.secondary)
                        Stepper(value: $lightIntensity, in: 100...10000, step: 100) {
                            Text("\(Int(lightIntensity)) lm")
                                .font(.caption)
                                .monospacedDigit()
                        }
                    }

                    // Wall color (only for room)
                    if sceneType == .room {
                        Divider().frame(height: 50)

                        VStack(alignment: .leading, spacing: 4) {
                            Text("Walls").font(.caption).foregroundStyle(.secondary)
                            Picker("", selection: $wallColor) {
                                ForEach(WallColor.allCases, id: \.self) { color in
                                    Text(color.rawValue).tag(color)
                                }
                            }
                            .pickerStyle(.segmented)
                            .frame(width: 180)
                        }
                    }

                    Divider().frame(height: 50)

                    // Toggles
                    VStack(alignment: .leading, spacing: 4) {
                        Toggle("Show Luminaire", isOn: $showLuminaire)
                            .font(.caption)
                        Toggle("Show LDC Solid", isOn: $showPhotometricSolid)
                            .font(.caption)
                    }
                }
                .padding(.horizontal)
                .padding(.vertical, 8)
            }
            .background(Color(white: isDarkTheme ? 0.15 : 0.95))
        }
    }

}

// MARK: - SceneKit Room View

#if os(macOS)

/// Custom SCNView that handles keyboard events for WASD navigation
class KeyboardSCNView: SCNView {
    weak var coordinator: RoomSceneView.Coordinator?

    override var acceptsFirstResponder: Bool { true }
    override var canBecomeKeyView: Bool { true }

    override func keyDown(with event: NSEvent) {
        let movementKeys: Set<UInt16> = [0, 1, 2, 13, 12, 14, 123, 124, 125, 126] // A,S,D,W,Q,E,arrows
        print("KeyDown: \(event.keyCode), isMovement: \(movementKeys.contains(event.keyCode))")
        if movementKeys.contains(event.keyCode) && !event.isARepeat {
            coordinator?.handleKeyDown(event.keyCode)
        } else {
            super.keyDown(with: event)
        }
    }

    override func keyUp(with event: NSEvent) {
        let movementKeys: Set<UInt16> = [0, 1, 2, 13, 12, 14, 123, 124, 125, 126]
        if movementKeys.contains(event.keyCode) {
            coordinator?.handleKeyUp(event.keyCode)
        } else {
            super.keyUp(with: event)
        }
    }

    override func mouseDown(with event: NSEvent) {
        // Become first responder on click to receive keyboard events
        window?.makeFirstResponder(self)
        super.mouseDown(with: event)
    }

    // Handle scroll wheel for walking forward/backward as alternative
    override func scrollWheel(with event: NSEvent) {
        guard let coordinator = coordinator,
              let cameraNode = scene?.rootNode.childNode(withName: "camera", recursively: false) else {
            print("ScrollWheel: no coordinator or camera")
            super.scrollWheel(with: event)
            return
        }

        print("ScrollWheel: deltaY=\(event.scrollingDeltaY), pos=\(cameraNode.simdPosition)")
        let scrollSpeed: Float = 0.1
        let forward = SIMD3<Float>(-sin(coordinator.yaw), 0, -cos(coordinator.yaw))
        var newPos = cameraNode.simdPosition + forward * Float(event.scrollingDeltaY) * scrollSpeed

        // Only clamp for indoor room scenes - outdoor scenes have no walls
        if coordinator.parent.sceneType == .room {
            let margin: Float = 0.2
            let maxX = Float(coordinator.parent.roomWidth) - margin
            let maxZ = Float(coordinator.parent.roomLength) - margin
            let maxY = Float(coordinator.parent.roomHeight) - 0.2

            newPos.x = max(margin, min(maxX, newPos.x))
            newPos.z = max(margin, min(maxZ, newPos.z))
            newPos.y = max(0.3, min(maxY, newPos.y))
        } else {
            // Outdoor: just keep above ground
            newPos.y = max(0.3, newPos.y)
        }

        cameraNode.simdPosition = newPos
    }
}

struct RoomSceneView: NSViewRepresentable {
    typealias NSViewType = KeyboardSCNView

    let ldt: Eulumdat
    let sceneType: SceneType
    let roomWidth: Double
    let roomLength: Double
    let roomHeight: Double
    let mountingHeight: Double
    let lightIntensity: Double
    let colorTemperature: Double
    let showLuminaire: Bool
    let showPhotometricSolid: Bool
    let wallColor: Room3DView.WallColor
    let isDarkTheme: Bool

    class Coordinator: NSObject {
        var parent: RoomSceneView
        weak var scnView: SCNView?
        var yaw: Float = 0
        var pitch: Float = 0
        var pressedKeys: Set<UInt16> = []
        var moveTimer: Timer?

        init(_ parent: RoomSceneView) {
            self.parent = parent
        }

        deinit {
            moveTimer?.invalidate()
        }

        @objc func handlePan(_ gesture: NSPanGestureRecognizer) {
            guard let scnView = scnView, let cameraNode = scnView.scene?.rootNode.childNode(withName: "camera", recursively: false) else { return }

            let translation = gesture.translation(in: gesture.view)
            let sensitivity: Float = 0.003

            yaw -= Float(translation.x) * sensitivity
            pitch -= Float(translation.y) * sensitivity
            pitch = max(-.pi / 2.2, min(.pi / 2.2, pitch))

            cameraNode.eulerAngles = SCNVector3(pitch, yaw, 0)
            gesture.setTranslation(.zero, in: gesture.view)
        }

        @objc func handleDoubleClick(_ gesture: NSClickGestureRecognizer) {
            guard let scnView = scnView,
                  let cameraNode = scnView.scene?.rootNode.childNode(withName: "camera", recursively: false) else { return }

            // Hit test to find where user clicked
            let location = gesture.location(in: scnView)
            let hitResults = scnView.hitTest(location, options: [.searchMode: SCNHitTestSearchMode.closest.rawValue])

            if let hit = hitResults.first {
                // Teleport to clicked position (at eye level)
                var targetPos = hit.worldCoordinates
                targetPos.y = cameraNode.position.y  // Keep same height

                // Only clamp for indoor room scenes
                if parent.sceneType == .room {
                    let margin: CGFloat = 0.2
                    let maxX = CGFloat(parent.roomWidth) - margin
                    let maxZ = CGFloat(parent.roomLength) - margin
                    let maxY = CGFloat(parent.roomHeight) - 0.2

                    targetPos.x = max(margin, min(maxX, targetPos.x))
                    targetPos.z = max(margin, min(maxZ, targetPos.z))
                    targetPos.y = max(0.3, min(maxY, targetPos.y))
                } else {
                    // Outdoor: just keep above ground
                    targetPos.y = max(0.3, targetPos.y)
                }

                // Animate movement
                SCNTransaction.begin()
                SCNTransaction.animationDuration = 0.3
                cameraNode.position = targetPos
                SCNTransaction.commit()
            }
        }

        func handleKeyDown(_ keyCode: UInt16) {
            pressedKeys.insert(keyCode)
            startMoveTimerIfNeeded()
        }

        func handleKeyUp(_ keyCode: UInt16) {
            pressedKeys.remove(keyCode)
            if pressedKeys.isEmpty {
                moveTimer?.invalidate()
                moveTimer = nil
            }
        }

        func startMoveTimerIfNeeded() {
            guard moveTimer == nil else { return }
            moveTimer = Timer.scheduledTimer(withTimeInterval: 1.0/60.0, repeats: true) { [weak self] _ in
                self?.processMovement()
            }
        }

        func processMovement() {
            guard let scnView = scnView,
                  let cameraNode = scnView.scene?.rootNode.childNode(withName: "camera", recursively: false) else { return }

            let moveSpeed: Float = 0.15
            var movement = SIMD3<Float>(0, 0, 0)

            // Get camera's forward and right vectors (in world space, ignoring pitch)
            let forward = SIMD3<Float>(-sin(yaw), 0, -cos(yaw))
            let right = SIMD3<Float>(cos(yaw), 0, -sin(yaw))

            // WASD keys (using key codes)
            // W = 13, A = 0, S = 1, D = 2, Arrow keys: Up=126, Down=125, Left=123, Right=124
            if pressedKeys.contains(13) || pressedKeys.contains(126) { movement += forward }  // W or Up
            if pressedKeys.contains(1) || pressedKeys.contains(125) { movement -= forward }   // S or Down
            if pressedKeys.contains(0) || pressedKeys.contains(123) { movement -= right }     // A or Left
            if pressedKeys.contains(2) || pressedKeys.contains(124) { movement += right }     // D or Right
            if pressedKeys.contains(12) { movement.y += 1 }  // Q - up
            if pressedKeys.contains(14) { movement.y -= 1 }  // E - down

            if simd_length(movement) > 0 {
                movement = simd_normalize(movement) * moveSpeed
                var newPos = cameraNode.simdPosition + movement

                // Only clamp for indoor room scenes - outdoor scenes have no walls
                if parent.sceneType == .room {
                    let margin: Float = 0.2
                    let maxX = Float(parent.roomWidth) - margin
                    let maxZ = Float(parent.roomLength) - margin
                    let maxY = Float(parent.roomHeight) - 0.2

                    newPos.x = max(margin, min(maxX, newPos.x))
                    newPos.z = max(margin, min(maxZ, newPos.z))
                    newPos.y = max(0.3, min(maxY, newPos.y))
                } else {
                    // Outdoor: just keep above ground
                    newPos.y = max(0.3, newPos.y)
                }

                cameraNode.simdPosition = newPos
            }
        }
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(self)
    }

    func makeNSView(context: Context) -> KeyboardSCNView {
        let scnView = KeyboardSCNView()
        scnView.coordinator = context.coordinator
        scnView.scene = createScene()
        scnView.allowsCameraControl = false
        scnView.autoenablesDefaultLighting = false
        scnView.backgroundColor = isDarkTheme ? NSColor(white: 0.1, alpha: 1) : NSColor(white: 0.2, alpha: 1)
        scnView.antialiasingMode = .multisampling4X

        context.coordinator.scnView = scnView
        context.coordinator.parent = self

        // Initialize yaw/pitch from camera orientation
        if let camera = scnView.scene?.rootNode.childNode(withName: "camera", recursively: false) {
            context.coordinator.pitch = Float(camera.eulerAngles.x)
            context.coordinator.yaw = Float(camera.eulerAngles.y)
        }

        // Pan gesture for looking around
        let panGesture = NSPanGestureRecognizer(target: context.coordinator, action: #selector(Coordinator.handlePan(_:)))
        scnView.addGestureRecognizer(panGesture)

        // Double-click to teleport
        let doubleClickGesture = NSClickGestureRecognizer(target: context.coordinator, action: #selector(Coordinator.handleDoubleClick(_:)))
        doubleClickGesture.numberOfClicksRequired = 2
        scnView.addGestureRecognizer(doubleClickGesture)

        return scnView
    }

    func updateNSView(_ scnView: KeyboardSCNView, context: Context) {
        // Preserve camera position and orientation when scene updates
        let existingCamera = scnView.scene?.rootNode.childNode(withName: "camera", recursively: false)
        let savedPosition = existingCamera?.position
        let savedEulerAngles = existingCamera?.eulerAngles

        scnView.scene = createScene()
        scnView.backgroundColor = isDarkTheme ? NSColor(white: 0.1, alpha: 1) : NSColor(white: 0.2, alpha: 1)

        // Restore camera state
        if let newCamera = scnView.scene?.rootNode.childNode(withName: "camera", recursively: false) {
            if let pos = savedPosition {
                // Clamp restored position to new room bounds
                let margin: CGFloat = 0.3
                let clampedX = max(margin, min(CGFloat(roomWidth) - margin, pos.x))
                let clampedZ = max(margin, min(CGFloat(roomLength) - margin, pos.z))
                let clampedY = max(0.5, min(CGFloat(roomHeight) - 0.3, pos.y))
                newCamera.position = SCNVector3(clampedX, clampedY, clampedZ)
            }
            if let angles = savedEulerAngles {
                newCamera.eulerAngles = angles
                context.coordinator.pitch = Float(angles.x)
                context.coordinator.yaw = Float(angles.y)
            }
        }
    }

    private func createScene() -> SCNScene {
        let builder = RoomSceneBuilder(
            ldt: ldt,
            sceneType: sceneType,
            roomWidth: roomWidth,
            roomLength: roomLength,
            roomHeight: roomHeight,
            mountingHeight: mountingHeight,
            lightIntensity: lightIntensity,
            colorTemperature: colorTemperature,
            showLuminaire: showLuminaire,
            showPhotometricSolid: showPhotometricSolid,
            wallColor: wallColor,
            isDarkTheme: isDarkTheme
        )
        return builder.build()
    }
}
#endif

#if os(iOS)
struct RoomSceneView: UIViewRepresentable {
    let ldt: Eulumdat
    let sceneType: SceneType
    let roomWidth: Double
    let roomLength: Double
    let roomHeight: Double
    let mountingHeight: Double
    let lightIntensity: Double
    let colorTemperature: Double
    let showLuminaire: Bool
    let showPhotometricSolid: Bool
    let wallColor: Room3DView.WallColor
    let isDarkTheme: Bool

    class Coordinator: NSObject {
        var parent: RoomSceneView
        weak var scnView: SCNView?
        var yaw: Float = 0
        var pitch: Float = 0
        var lastPanLocation: CGPoint = .zero

        init(_ parent: RoomSceneView) {
            self.parent = parent
        }

        @objc func handlePan(_ gesture: UIPanGestureRecognizer) {
            guard let scnView = scnView, let cameraNode = scnView.scene?.rootNode.childNode(withName: "camera", recursively: false) else { return }

            let translation = gesture.translation(in: gesture.view)
            let sensitivity: Float = 0.003

            yaw -= Float(translation.x) * sensitivity
            pitch -= Float(translation.y) * sensitivity
            pitch = max(-.pi / 2.2, min(.pi / 2.2, pitch))

            cameraNode.eulerAngles = SCNVector3(pitch, yaw, 0)
            gesture.setTranslation(.zero, in: gesture.view)
        }

        @objc func handlePinch(_ gesture: UIPinchGestureRecognizer) {
            guard let scnView = scnView, let cameraNode = scnView.scene?.rootNode.childNode(withName: "camera", recursively: false) else { return }

            if gesture.state == .changed {
                let moveSpeed: Float = Float(gesture.scale - 1.0) * 2.0
                let forward = cameraNode.simdWorldFront
                var newPos = cameraNode.simdPosition + forward * moveSpeed

                let margin: Float = 0.3
                newPos.x = max(margin, min(Float(parent.roomWidth) - margin, newPos.x))
                newPos.z = max(margin, min(Float(parent.roomLength) - margin, newPos.z))
                newPos.y = max(0.5, min(Float(parent.roomHeight) - 0.3, newPos.y))

                cameraNode.simdPosition = newPos
                gesture.scale = 1.0
            }
        }
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(self)
    }

    func makeUIView(context: Context) -> SCNView {
        let scnView = SCNView()
        scnView.scene = createScene()
        scnView.allowsCameraControl = false
        scnView.autoenablesDefaultLighting = false
        scnView.backgroundColor = isDarkTheme ? UIColor(white: 0.1, alpha: 1) : UIColor(white: 0.2, alpha: 1)
        scnView.antialiasingMode = .multisampling4X

        context.coordinator.scnView = scnView

        // Pan for looking around
        let panGesture = UIPanGestureRecognizer(target: context.coordinator, action: #selector(Coordinator.handlePan(_:)))
        scnView.addGestureRecognizer(panGesture)

        // Pinch for walking forward/backward
        let pinchGesture = UIPinchGestureRecognizer(target: context.coordinator, action: #selector(Coordinator.handlePinch(_:)))
        scnView.addGestureRecognizer(pinchGesture)

        return scnView
    }

    func updateUIView(_ scnView: SCNView, context: Context) {
        let existingCamera = scnView.scene?.rootNode.childNode(withName: "camera", recursively: false)
        let savedPosition = existingCamera?.position
        let savedEulerAngles = existingCamera?.eulerAngles

        scnView.scene = createScene()
        scnView.backgroundColor = isDarkTheme ? UIColor(white: 0.1, alpha: 1) : UIColor(white: 0.2, alpha: 1)

        if let newCamera = scnView.scene?.rootNode.childNode(withName: "camera", recursively: false) {
            if let pos = savedPosition {
                var clampedPos = pos
                let margin: Float = 0.3
                clampedPos.x = max(margin, min(Float(roomWidth) - margin, clampedPos.x))
                clampedPos.z = max(margin, min(Float(roomLength) - margin, clampedPos.z))
                clampedPos.y = max(0.5, min(Float(roomHeight) - 0.3, clampedPos.y))
                newCamera.position = clampedPos
            }
            if let angles = savedEulerAngles {
                newCamera.eulerAngles = angles
                context.coordinator.pitch = angles.x
                context.coordinator.yaw = angles.y
            }
        }
    }

    private func createScene() -> SCNScene {
        let builder = RoomSceneBuilder(
            ldt: ldt,
            sceneType: sceneType,
            roomWidth: roomWidth,
            roomLength: roomLength,
            roomHeight: roomHeight,
            mountingHeight: mountingHeight,
            lightIntensity: lightIntensity,
            colorTemperature: colorTemperature,
            showLuminaire: showLuminaire,
            showPhotometricSolid: showPhotometricSolid,
            wallColor: wallColor,
            isDarkTheme: isDarkTheme
        )
        return builder.build()
    }
}
#endif

// MARK: - Room Scene Builder

class RoomSceneBuilder {
    let ldt: Eulumdat
    let sceneType: SceneType
    let roomWidth: Double
    let roomLength: Double
    let roomHeight: Double
    let mountingHeight: Double
    let lightIntensity: Double
    let colorTemperature: Double
    let showLuminaire: Bool
    let showPhotometricSolid: Bool
    let wallColor: Room3DView.WallColor
    let isDarkTheme: Bool

    private var iesFileURL: URL?

    init(ldt: Eulumdat, sceneType: SceneType, roomWidth: Double, roomLength: Double, roomHeight: Double,
         mountingHeight: Double, lightIntensity: Double, colorTemperature: Double,
         showLuminaire: Bool, showPhotometricSolid: Bool, wallColor: Room3DView.WallColor, isDarkTheme: Bool) {
        self.ldt = ldt
        self.sceneType = sceneType
        self.roomWidth = roomWidth
        self.roomLength = roomLength
        self.roomHeight = roomHeight
        self.mountingHeight = mountingHeight
        self.lightIntensity = lightIntensity
        self.colorTemperature = colorTemperature
        self.showLuminaire = showLuminaire
        self.showPhotometricSolid = showPhotometricSolid
        self.wallColor = wallColor
        self.isDarkTheme = isDarkTheme
    }

    /// Convert color temperature (Kelvin) to RGB color
    /// Using Tanner Helland's algorithm: http://www.tannerhelland.com/4435/convert-temperature-rgb-algorithm-code/
    private func colorFromTemperature(_ kelvin: Double) -> Any {
        let temp = kelvin / 100.0
        var r: Double, g: Double, b: Double

        // Red
        if temp <= 66 {
            r = 255
        } else {
            r = temp - 60
            r = 329.698727446 * pow(r, -0.1332047592)
            r = max(0, min(255, r))
        }

        // Green
        if temp <= 66 {
            g = temp
            g = 99.4708025861 * log(g) - 161.1195681661
            g = max(0, min(255, g))
        } else {
            g = temp - 60
            g = 288.1221695283 * pow(g, -0.0755148492)
            g = max(0, min(255, g))
        }

        // Blue
        if temp >= 66 {
            b = 255
        } else if temp <= 19 {
            b = 0
        } else {
            b = temp - 10
            b = 138.5177312231 * log(b) - 305.0447927307
            b = max(0, min(255, b))
        }

        #if os(macOS)
        return NSColor(red: r / 255.0, green: g / 255.0, blue: b / 255.0, alpha: 1.0)
        #else
        return UIColor(red: r / 255.0, green: g / 255.0, blue: b / 255.0, alpha: 1.0)
        #endif
    }

    func build() -> SCNScene {
        let scene = SCNScene()

        // Camera setup - first-person view
        let cameraNode = SCNNode()
        cameraNode.name = "camera"
        cameraNode.camera = SCNCamera()
        cameraNode.camera?.zNear = 0.1
        cameraNode.camera?.zFar = 200
        cameraNode.camera?.fieldOfView = 75
        cameraNode.camera?.wantsHDR = true
        cameraNode.camera?.wantsExposureAdaptation = true

        // Different camera setup and exposure for different scenes
        switch sceneType {
        case .room:
            cameraNode.camera?.exposureOffset = 0.5
            let eyeHeight = min(1.6, roomHeight * 0.5)
            cameraNode.position = SCNVector3(0.5, Float(eyeHeight), 0.5)
        case .road:
            cameraNode.camera?.exposureOffset = -0.5  // Night scene, darker
            cameraNode.position = SCNVector3(Float(roomWidth / 2), 1.7, 2.0)
        case .parking:
            cameraNode.camera?.exposureOffset = -0.3
            cameraNode.position = SCNVector3(Float(roomWidth / 4), 1.7, Float(roomLength / 4))
        case .outdoor:
            cameraNode.camera?.exposureOffset = 0.0
            cameraNode.position = SCNVector3(Float(roomWidth / 3), 1.7, Float(roomLength / 3))
        }

        // Look at the luminaire
        cameraNode.look(at: SCNVector3(
            Float(roomWidth / 2),
            Float(mountingHeight),
            Float(roomLength / 2)
        ))

        scene.rootNode.addChildNode(cameraNode)

        // Build scene geometry based on type
        switch sceneType {
        case .room:
            addRoom(to: scene.rootNode)
        case .road:
            addRoadScene(to: scene.rootNode)
        case .parking:
            addParkingScene(to: scene.rootNode)
        case .outdoor:
            addOutdoorScene(to: scene.rootNode)
        }

        // Add IES light
        addIESLight(to: scene.rootNode)

        // Add luminaire representation
        if showLuminaire {
            addLuminaireModel(to: scene.rootNode)
        }

        // Add small photometric solid preview
        if showPhotometricSolid {
            addPhotometricSolid(to: scene.rootNode)
        }

        // Ambient light - less for outdoor, more for room
        let ambientLight = SCNNode()
        ambientLight.light = SCNLight()
        ambientLight.light?.type = .ambient
        ambientLight.light?.intensity = sceneType == .room ? 200 : 50  // Darker ambient for outdoor
        #if os(macOS)
        ambientLight.light?.color = sceneType == .room ? NSColor(white: 0.8, alpha: 1) : NSColor(red: 0.1, green: 0.1, blue: 0.2, alpha: 1)
        #else
        ambientLight.light?.color = sceneType == .room ? UIColor(white: 0.8, alpha: 1) : UIColor(red: 0.1, green: 0.1, blue: 0.2, alpha: 1)
        #endif
        scene.rootNode.addChildNode(ambientLight)

        // Directional light (moonlight for outdoor scenes)
        let dirLight = SCNNode()
        dirLight.light = SCNLight()
        dirLight.light?.type = .directional
        dirLight.light?.intensity = sceneType == .room ? 300 : 50
        #if os(macOS)
        dirLight.light?.color = sceneType == .room ? NSColor.white : NSColor(red: 0.4, green: 0.4, blue: 0.6, alpha: 1)
        #else
        dirLight.light?.color = sceneType == .room ? UIColor.white : UIColor(red: 0.4, green: 0.4, blue: 0.6, alpha: 1)
        #endif
        dirLight.position = SCNVector3(Float(roomWidth/2), Float(mountingHeight + 10), Float(roomLength/2))
        dirLight.look(at: SCNVector3(Float(roomWidth/2), 0, Float(roomLength/2)))
        scene.rootNode.addChildNode(dirLight)

        return scene
    }

    private func addRoom(to parent: SCNNode) {
        let colors = wallColor.color

        // Floor
        let floor = SCNBox(width: CGFloat(roomWidth), height: 0.02, length: CGFloat(roomLength), chamferRadius: 0)
        let floorMaterial = SCNMaterial()
        floorMaterial.diffuse.contents = colors.floor
        floorMaterial.lightingModel = .physicallyBased
        floorMaterial.roughness.contents = 0.8
        floorMaterial.metalness.contents = 0.0
        floor.materials = [floorMaterial]
        let floorNode = SCNNode(geometry: floor)
        floorNode.position = SCNVector3(Float(roomWidth / 2), 0.01, Float(roomLength / 2))
        parent.addChildNode(floorNode)

        // Ceiling
        let ceiling = SCNBox(width: CGFloat(roomWidth), height: 0.02, length: CGFloat(roomLength), chamferRadius: 0)
        let ceilingMaterial = SCNMaterial()
        ceilingMaterial.diffuse.contents = colors.wall
        ceilingMaterial.lightingModel = .physicallyBased
        ceilingMaterial.roughness.contents = 0.9
        ceiling.materials = [ceilingMaterial]
        let ceilingNode = SCNNode(geometry: ceiling)
        ceilingNode.position = SCNVector3(Float(roomWidth / 2), Float(roomHeight), Float(roomLength / 2))
        parent.addChildNode(ceilingNode)

        // Walls
        addWall(to: parent, width: roomWidth, height: roomHeight,
                position: SCNVector3(Float(roomWidth / 2), Float(roomHeight / 2), 0),
                rotation: SCNVector4(0, 0, 0, 0), color: colors.wall)

        addWall(to: parent, width: roomWidth, height: roomHeight,
                position: SCNVector3(Float(roomWidth / 2), Float(roomHeight / 2), Float(roomLength)),
                rotation: SCNVector4(0, 1, 0, Float.pi), color: colors.wall)

        addWall(to: parent, width: roomLength, height: roomHeight,
                position: SCNVector3(0, Float(roomHeight / 2), Float(roomLength / 2)),
                rotation: SCNVector4(0, 1, 0, Float.pi / 2), color: colors.wall)

        addWall(to: parent, width: roomLength, height: roomHeight,
                position: SCNVector3(Float(roomWidth), Float(roomHeight / 2), Float(roomLength / 2)),
                rotation: SCNVector4(0, 1, 0, -Float.pi / 2), color: colors.wall)
    }

    private func addWall(to parent: SCNNode, width: Double, height: Double,
                        position: SCNVector3, rotation: SCNVector4, color: Any) {
        let wall = SCNBox(width: CGFloat(width), height: CGFloat(height), length: 0.02, chamferRadius: 0)
        let wallMaterial = SCNMaterial()
        wallMaterial.diffuse.contents = color
        wallMaterial.lightingModel = .physicallyBased
        wallMaterial.roughness.contents = 0.9
        wallMaterial.metalness.contents = 0.0
        wall.materials = [wallMaterial]
        let wallNode = SCNNode(geometry: wall)
        wallNode.position = position
        wallNode.rotation = rotation
        parent.addChildNode(wallNode)
    }

    // MARK: - Road Scene

    private func addRoadScene(to parent: SCNNode) {
        // Dark asphalt road
        let road = SCNBox(width: CGFloat(roomWidth), height: 0.02, length: CGFloat(roomLength), chamferRadius: 0)
        let roadMaterial = SCNMaterial()
        #if os(macOS)
        roadMaterial.diffuse.contents = NSColor(white: 0.15, alpha: 1)
        #else
        roadMaterial.diffuse.contents = UIColor(white: 0.15, alpha: 1)
        #endif
        roadMaterial.lightingModel = .physicallyBased
        roadMaterial.roughness.contents = 0.9
        road.materials = [roadMaterial]
        let roadNode = SCNNode(geometry: road)
        roadNode.position = SCNVector3(Float(roomWidth / 2), 0.01, Float(roomLength / 2))
        parent.addChildNode(roadNode)

        // Road markings (center line)
        for i in stride(from: 2.0, to: roomLength - 2, by: 4.0) {
            let marking = SCNBox(width: 0.15, height: 0.03, length: 2.0, chamferRadius: 0)
            let markingMaterial = SCNMaterial()
            #if os(macOS)
            markingMaterial.diffuse.contents = NSColor.white
            markingMaterial.emission.contents = NSColor(white: 0.3, alpha: 1)
            #else
            markingMaterial.diffuse.contents = UIColor.white
            markingMaterial.emission.contents = UIColor(white: 0.3, alpha: 1)
            #endif
            marking.materials = [markingMaterial]
            let markingNode = SCNNode(geometry: marking)
            markingNode.position = SCNVector3(Float(roomWidth / 2), 0.02, Float(i))
            parent.addChildNode(markingNode)
        }

        // Sidewalks on both sides
        addSidewalk(to: parent, xPos: 0.5, length: roomLength)
        addSidewalk(to: parent, xPos: roomWidth - 0.5, length: roomLength)

        // Light pole
        addPole(to: parent, position: SCNVector3(Float(roomWidth / 2 + 0.3), 0, Float(roomLength / 2)))
    }

    // MARK: - Parking Scene

    private func addParkingScene(to parent: SCNNode) {
        // Parking lot surface
        let lot = SCNBox(width: CGFloat(roomWidth), height: 0.02, length: CGFloat(roomLength), chamferRadius: 0)
        let lotMaterial = SCNMaterial()
        #if os(macOS)
        lotMaterial.diffuse.contents = NSColor(white: 0.2, alpha: 1)
        #else
        lotMaterial.diffuse.contents = UIColor(white: 0.2, alpha: 1)
        #endif
        lotMaterial.lightingModel = .physicallyBased
        lotMaterial.roughness.contents = 0.85
        lot.materials = [lotMaterial]
        let lotNode = SCNNode(geometry: lot)
        lotNode.position = SCNVector3(Float(roomWidth / 2), 0.01, Float(roomLength / 2))
        parent.addChildNode(lotNode)

        // Parking space lines
        let spaceWidth = 2.5
        let spaceLength = 5.0
        for row in stride(from: 3.0, to: roomLength - 3, by: spaceLength + 1) {
            for col in stride(from: spaceWidth, to: roomWidth - 1, by: spaceWidth) {
                addParkingLine(to: parent, x: col, z: row, length: spaceLength)
            }
        }

        // Light pole in center
        addPole(to: parent, position: SCNVector3(Float(roomWidth / 2), 0, Float(roomLength / 2)))
    }

    // MARK: - Outdoor/Garden Scene

    private func addOutdoorScene(to parent: SCNNode) {
        // Grass ground
        let grass = SCNBox(width: CGFloat(roomWidth), height: 0.02, length: CGFloat(roomLength), chamferRadius: 0)
        let grassMaterial = SCNMaterial()
        #if os(macOS)
        grassMaterial.diffuse.contents = NSColor(red: 0.15, green: 0.3, blue: 0.1, alpha: 1)
        #else
        grassMaterial.diffuse.contents = UIColor(red: 0.15, green: 0.3, blue: 0.1, alpha: 1)
        #endif
        grassMaterial.lightingModel = .physicallyBased
        grassMaterial.roughness.contents = 0.95
        grass.materials = [grassMaterial]
        let grassNode = SCNNode(geometry: grass)
        grassNode.position = SCNVector3(Float(roomWidth / 2), 0.01, Float(roomLength / 2))
        parent.addChildNode(grassNode)

        // Garden path
        let path = SCNBox(width: 1.2, height: 0.03, length: CGFloat(roomLength - 2), chamferRadius: 0)
        let pathMaterial = SCNMaterial()
        #if os(macOS)
        pathMaterial.diffuse.contents = NSColor(white: 0.5, alpha: 1)
        #else
        pathMaterial.diffuse.contents = UIColor(white: 0.5, alpha: 1)
        #endif
        pathMaterial.lightingModel = .physicallyBased
        pathMaterial.roughness.contents = 0.8
        path.materials = [pathMaterial]
        let pathNode = SCNNode(geometry: path)
        pathNode.position = SCNVector3(Float(roomWidth / 2), 0.02, Float(roomLength / 2))
        parent.addChildNode(pathNode)

        // Add some simple bush shapes
        addBush(to: parent, position: SCNVector3(2, 0.4, 3))
        addBush(to: parent, position: SCNVector3(Float(roomWidth) - 2, 0.3, Float(roomLength) - 4))
        addBush(to: parent, position: SCNVector3(1.5, 0.35, Float(roomLength) - 2))

        // Light pole
        addPole(to: parent, position: SCNVector3(Float(roomWidth / 2), 0, Float(roomLength / 2)))
    }

    // MARK: - Scene Helper Methods

    private func addSidewalk(to parent: SCNNode, xPos: Double, length: Double) {
        let sidewalk = SCNBox(width: 1.0, height: 0.1, length: CGFloat(length), chamferRadius: 0)
        let material = SCNMaterial()
        #if os(macOS)
        material.diffuse.contents = NSColor(white: 0.6, alpha: 1)
        #else
        material.diffuse.contents = UIColor(white: 0.6, alpha: 1)
        #endif
        material.lightingModel = .physicallyBased
        material.roughness.contents = 0.8
        sidewalk.materials = [material]
        let node = SCNNode(geometry: sidewalk)
        node.position = SCNVector3(Float(xPos), 0.05, Float(length / 2))
        parent.addChildNode(node)
    }

    private func addPole(to parent: SCNNode, position: SCNVector3) {
        // Pole
        let pole = SCNCylinder(radius: 0.08, height: CGFloat(mountingHeight - 0.3))
        let poleMaterial = SCNMaterial()
        #if os(macOS)
        poleMaterial.diffuse.contents = NSColor(white: 0.4, alpha: 1)
        #else
        poleMaterial.diffuse.contents = UIColor(white: 0.4, alpha: 1)
        #endif
        poleMaterial.lightingModel = .physicallyBased
        poleMaterial.metalness.contents = 0.6
        poleMaterial.roughness.contents = 0.4
        pole.materials = [poleMaterial]
        let poleNode = SCNNode(geometry: pole)
        poleNode.position = SCNVector3(CGFloat(position.x), CGFloat(mountingHeight / 2), CGFloat(position.z))
        parent.addChildNode(poleNode)

        // Arm extending toward light position
        let arm = SCNCylinder(radius: 0.05, height: 1.0)
        arm.materials = [poleMaterial]
        let armNode = SCNNode(geometry: arm)
        armNode.position = SCNVector3(CGFloat(position.x) - 0.3, CGFloat(mountingHeight - 0.2), CGFloat(position.z))
        armNode.eulerAngles.z = .pi / 2  // Horizontal
        parent.addChildNode(armNode)
    }

    private func addParkingLine(to parent: SCNNode, x: Double, z: Double, length: Double) {
        let line = SCNBox(width: 0.1, height: 0.02, length: CGFloat(length), chamferRadius: 0)
        let material = SCNMaterial()
        #if os(macOS)
        material.diffuse.contents = NSColor.white
        material.emission.contents = NSColor(white: 0.2, alpha: 1)
        #else
        material.diffuse.contents = UIColor.white
        material.emission.contents = UIColor(white: 0.2, alpha: 1)
        #endif
        line.materials = [material]
        let node = SCNNode(geometry: line)
        node.position = SCNVector3(Float(x), 0.02, Float(z))
        parent.addChildNode(node)
    }

    private func addBush(to parent: SCNNode, position: SCNVector3) {
        let bush = SCNSphere(radius: CGFloat(position.y))
        let material = SCNMaterial()
        #if os(macOS)
        material.diffuse.contents = NSColor(red: 0.1, green: 0.25, blue: 0.05, alpha: 1)
        #else
        material.diffuse.contents = UIColor(red: 0.1, green: 0.25, blue: 0.05, alpha: 1)
        #endif
        material.lightingModel = .physicallyBased
        material.roughness.contents = 0.95
        bush.materials = [material]
        let node = SCNNode(geometry: bush)
        node.position = position
        parent.addChildNode(node)
    }

    private func addIESLight(to parent: SCNNode) {
        // Export LDT to IES and save to temp file
        let iesContent = exportIes(ldt: ldt)
        let tempDir = FileManager.default.temporaryDirectory
        let iesURL = tempDir.appendingPathComponent("luminaire_\(UUID().uuidString).ies")

        do {
            try iesContent.write(to: iesURL, atomically: true, encoding: .utf8)
            self.iesFileURL = iesURL

            // Create IES light
            let light = SCNLight()
            light.type = .IES
            light.iesProfileURL = iesURL
            light.intensity = CGFloat(lightIntensity)
            // Apply color temperature from lamp data
            light.color = colorFromTemperature(colorTemperature)
            light.castsShadow = true
            light.shadowMode = .deferred
            light.shadowSampleCount = 8
            light.shadowRadius = 2

            let lightNode = SCNNode()
            lightNode.light = light
            lightNode.position = SCNVector3(
                Float(roomWidth / 2),
                Float(mountingHeight),
                Float(roomLength / 2)
            )
            // IES lights typically point down (negative Y)
            lightNode.eulerAngles.x = 0 // Already pointing down by default

            parent.addChildNode(lightNode)
        } catch {
            print("Failed to create IES file: \(error)")
            // Fallback to spot light
            addFallbackLight(to: parent)
        }
    }

    private func addFallbackLight(to parent: SCNNode) {
        let light = SCNLight()
        light.type = .spot
        light.intensity = CGFloat(lightIntensity)
        // Apply color temperature from lamp data
        light.color = colorFromTemperature(colorTemperature)
        light.spotInnerAngle = 30
        light.spotOuterAngle = 80
        light.castsShadow = true

        let lightNode = SCNNode()
        lightNode.light = light
        lightNode.position = SCNVector3(
            Float(roomWidth / 2),
            Float(mountingHeight),
            Float(roomLength / 2)
        )
        lightNode.eulerAngles.x = -.pi / 2 // Point down

        parent.addChildNode(lightNode)
    }

    private func addLuminaireModel(to parent: SCNNode) {
        // Simple luminaire representation based on dimensions
        let lumWidth = max(ldt.width / 1000.0, 0.1)  // Convert mm to m, min 10cm
        let lumLength = max(ldt.length / 1000.0, 0.1)
        let lumHeight = max(ldt.height / 1000.0, 0.05)

        let luminaire = SCNBox(
            width: CGFloat(lumWidth),
            height: CGFloat(lumHeight),
            length: CGFloat(lumLength),
            chamferRadius: 0.01
        )

        let material = SCNMaterial()
        // Use color temperature for luminaire emission glow
        let emissionColor = colorFromTemperature(colorTemperature)
        #if os(macOS)
        material.diffuse.contents = NSColor.darkGray
        material.emission.contents = emissionColor
        #else
        material.diffuse.contents = UIColor.darkGray
        material.emission.contents = emissionColor
        #endif
        material.lightingModel = .physicallyBased
        material.metalness.contents = 0.8
        material.roughness.contents = 0.3
        luminaire.materials = [material]

        let luminaireNode = SCNNode(geometry: luminaire)
        luminaireNode.position = SCNVector3(
            Float(roomWidth / 2),
            Float(mountingHeight + lumHeight / 2),
            Float(roomLength / 2)
        )

        parent.addChildNode(luminaireNode)
    }

    private func addPhotometricSolid(to parent: SCNNode) {
        // Add a small photometric solid below the luminaire for reference
        let scale: Float = 0.3 // Small scale

        let solidNode = SCNNode()
        solidNode.name = "photometricSolid"
        solidNode.position = SCNVector3(
            Float(roomWidth / 2),
            Float(mountingHeight - 0.1),
            Float(roomLength / 2)
        )
        solidNode.scale = SCNVector3(scale, scale, scale)

        // Build mesh using FFI sampling
        buildPhotometricMesh(parent: solidNode)

        parent.addChildNode(solidNode)
    }

    private func buildPhotometricMesh(parent: SCNNode) {
        let cStep: Double = 10.0
        let gStep: Double = 5.0

        let numC = Int(360.0 / cStep)
        let numG = Int(180.0 / gStep) + 1

        var vertices: [SCNVector3] = []
        var colors: [CGFloat] = []
        var indices: [Int32] = []

        for ci in 0..<numC {
            let cAngle = Double(ci) * cStep
            let cRad = Float(cAngle * .pi / 180.0)

            for gi in 0..<numG {
                let gAngle = Double(gi) * gStep
                let normalizedIntensity = sampleIntensityNormalized(ldt: ldt, cAngle: cAngle, gAngle: gAngle)
                let r = Float(normalizedIntensity)
                let gRad = Float(gAngle * .pi / 180.0)

                let x = r * sin(gRad) * cos(cRad)
                let z = r * sin(gRad) * sin(cRad)
                let y = -r * cos(gRad)

                vertices.append(SCNVector3(x, y, z))

                // Heatmap coloring
                let (cr, cg, cb) = heatmapColor(normalizedIntensity)
                colors.append(contentsOf: [CGFloat(cr), CGFloat(cg), CGFloat(cb), 0.7])
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

        let geometry = SCNGeometry(sources: [vertexSource, colorSource], elements: [element])

        let material = SCNMaterial()
        material.isDoubleSided = true
        material.lightingModel = .constant
        material.transparency = 0.7
        geometry.materials = [material]

        parent.addChildNode(SCNNode(geometry: geometry))
    }

    private func heatmapColor(_ value: Double) -> (Float, Float, Float) {
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

        return (r, g, b)
    }
}

// MARK: - Preview

#Preview {
    Room3DView(
        ldt: Eulumdat(
            identification: "Test",
            typeIndicator: .pointSourceSymmetric,
            symmetry: .verticalAxis,
            numCPlanes: 1,
            cPlaneDistance: 0,
            numGPlanes: 19,
            gPlaneDistance: 5,
            measurementReportNumber: "",
            luminaireName: "Test Downlight",
            luminaireNumber: "",
            fileName: "",
            dateUser: "",
            length: 200,
            width: 200,
            height: 100,
            luminousAreaLength: 150,
            luminousAreaWidth: 150,
            heightC0: 0,
            heightC90: 0,
            heightC180: 0,
            heightC270: 0,
            downwardFluxFraction: 100,
            lightOutputRatio: 85,
            conversionFactor: 1,
            tiltAngle: 0,
            lampSets: [],
            directRatios: [],
            cAngles: [0],
            gAngles: Array(stride(from: 0.0, through: 90.0, by: 5.0)),
            intensities: [[300, 295, 280, 260, 230, 190, 150, 110, 70, 40, 20, 10, 5, 2, 1, 0, 0, 0, 0]],
            maxIntensity: 300,
            totalLuminousFlux: 1000
        ),
        isDarkTheme: .constant(false)
    )
}
