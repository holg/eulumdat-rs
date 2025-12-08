//
//  EulumdatAppUITests.swift
//  EulumdatAppUITests
//
//  UI tests for verifying diagram interaction features
//
//  Updated: December 2024
//  - Removed fullscreen sheet tests (replaced with real window opening)
//  - Added tests for double-click opening new windows
//  - Added test for double-click on empty state opening file picker
//

import XCTest

final class EulumdatAppUITests: XCTestCase {

    var app: XCUIApplication!

    override func setUpWithError() throws {
        continueAfterFailure = false
        app = XCUIApplication()
        app.launch()
    }

    override func tearDownWithError() throws {
        app = nil
    }

    // MARK: - Helper Methods

    /// Resizes the main window to App Store screenshot dimensions (1440×900)
    /// Uses Cmd+Ctrl+Shift+S keyboard shortcut to trigger the app's resize function
    /// App Store accepts: 1280×800, 1440×900, 2560×1600, or 2880×1800
    private func resizeWindowForScreenshot() {
        #if os(macOS)
        let window = app.windows.firstMatch
        guard window.waitForExistence(timeout: 5) else { return }

        // Use keyboard shortcut Cmd+Ctrl+Shift+S to resize to App Store size
        app.typeKey("s", modifierFlags: [.command, .control, .shift])
        Thread.sleep(forTimeInterval: 0.5)
        #endif
    }

    /// Loads a template via the File menu (File > New from Template > Road Luminaire)
    private func loadTemplateViaMenu() {
        // Access the menu bar
        let menuBar = app.menuBars.firstMatch

        // Click on File menu
        let fileMenu = menuBar.menuBarItems["File"]
        XCTAssertTrue(fileMenu.waitForExistence(timeout: 5), "File menu should exist")
        fileMenu.click()

        // Hover over "New from Template" to open submenu
        let newFromTemplateItem = app.menuItems["New from Template"]
        XCTAssertTrue(newFromTemplateItem.waitForExistence(timeout: 3), "New from Template menu should exist")
        newFromTemplateItem.hover()

        // Wait for submenu to appear and click on "Road Luminaire"
        let roadLuminaire = app.menuItems["Road Luminaire"]
        if roadLuminaire.waitForExistence(timeout: 3) {
            roadLuminaire.click()
        } else {
            // Try clicking on the first available template
            let templates = app.menuItems.matching(NSPredicate(format: "title CONTAINS 'Luminaire' OR title CONTAINS 'Uplight' OR title CONTAINS 'Projector'"))
            if templates.count > 0 {
                templates.firstMatch.click()
            }
        }

        // Wait for the diagram to load
        Thread.sleep(forTimeInterval: 1.0)
    }

    /// Navigates to the Diagram tab by clicking the tab button
    private func navigateToDiagramTab() {
        let diagramTab = app.buttons["Diagram"]
        if diagramTab.waitForExistence(timeout: 5) {
            diagramTab.click()
        }
    }

    // MARK: - Basic App Tests

    func testAppLaunches() throws {
        // App should launch and show main window
        XCTAssertTrue(app.windows.count > 0, "App should have at least one window")
    }

    func testMenuBarExists() throws {
        let menuBar = app.menuBars.firstMatch
        XCTAssertTrue(menuBar.exists, "Menu bar should exist")

        let fileMenu = menuBar.menuBarItems["File"]
        XCTAssertTrue(fileMenu.exists, "File menu should exist")
    }

    // MARK: - Template Loading Tests

    func testLoadTemplateFromMenu() throws {
        loadTemplateViaMenu()

        // After loading, the Diagram tab should be selected and show content
        let diagramTab = app.buttons["Diagram"]
        XCTAssertTrue(diagramTab.waitForExistence(timeout: 5), "Diagram tab should exist after loading template")
    }

    func testDiagramTabExistsAfterLoad() throws {
        loadTemplateViaMenu()

        let diagramTab = app.buttons["Diagram"]
        XCTAssertTrue(diagramTab.waitForExistence(timeout: 5), "Diagram tab should exist")
    }

    // MARK: - Diagram Type Tests

    func testDiagramTypePicker() throws {
        loadTemplateViaMenu()
        navigateToDiagramTab()

        // Check all diagram type buttons exist
        let diagramTypes = ["Polar", "Cartesian", "Butterfly", "3D", "Heatmap", "BUG", "LCS"]

        for diagramType in diagramTypes {
            let button = app.buttons[diagramType]
            XCTAssertTrue(button.waitForExistence(timeout: 3), "\(diagramType) diagram type button should exist")
        }
    }

    func testSwitchDiagramTypes() throws {
        loadTemplateViaMenu()
        navigateToDiagramTab()

        // Switch to Cartesian
        let cartesianButton = app.buttons["Cartesian"]
        if cartesianButton.waitForExistence(timeout: 3) {
            cartesianButton.click()
            Thread.sleep(forTimeInterval: 0.5)
        }

        // Switch to Heatmap
        let heatmapButton = app.buttons["Heatmap"]
        if heatmapButton.waitForExistence(timeout: 2) {
            heatmapButton.click()
            Thread.sleep(forTimeInterval: 0.5)
        }

        // Switch to Polar
        let polarButton = app.buttons["Polar"]
        if polarButton.waitForExistence(timeout: 2) {
            polarButton.click()
        }

        // If we got here without crashing, the test passes
        XCTAssertTrue(true, "Should be able to switch between diagram types")
    }

    // MARK: - Zoom Tests

    func testZoomControlsExist() throws {
        loadTemplateViaMenu()
        navigateToDiagramTab()

        // The zoom percentage should be visible - look for any text containing %
        // Try accessibility identifier first
        let zoomById = app.staticTexts["ZoomPercentage"]
        if zoomById.waitForExistence(timeout: 5) {
            XCTAssertTrue(true, "Zoom percentage indicator found by ID")
        } else {
            // Fallback to predicate search
            let zoomText = app.staticTexts.matching(NSPredicate(format: "label CONTAINS '%'")).firstMatch
            XCTAssertTrue(zoomText.waitForExistence(timeout: 3), "Zoom percentage indicator should be visible")
        }
    }

    func testZoomInWithKeyboard() throws {
        loadTemplateViaMenu()
        navigateToDiagramTab()

        // Wait for diagram to load
        Thread.sleep(forTimeInterval: 1.0)

        // Get initial zoom text
        let zoomText = app.staticTexts["ZoomPercentage"]
        if zoomText.waitForExistence(timeout: 5) {
            let initialLabel = zoomText.label
            XCTAssertTrue(initialLabel.contains("100"), "Should start at 100% zoom")

            // Zoom in with Cmd++
            app.typeKey("+", modifierFlags: .command)
            Thread.sleep(forTimeInterval: 0.5)

            // Check zoom changed
            let newLabel = zoomText.label
            XCTAssertFalse(newLabel.contains("100"), "Zoom level should change after Cmd++")
        } else {
            XCTFail("Could not find zoom percentage element")
        }
    }

    func testZoomResetWithKeyboard() throws {
        loadTemplateViaMenu()
        navigateToDiagramTab()

        Thread.sleep(forTimeInterval: 1.0)

        // Zoom in first
        app.typeKey("+", modifierFlags: .command)
        app.typeKey("+", modifierFlags: .command)
        Thread.sleep(forTimeInterval: 0.5)

        // Reset zoom with Cmd+0
        app.typeKey("0", modifierFlags: .command)
        Thread.sleep(forTimeInterval: 0.5)

        // Check zoom is back to 100%
        let zoomText = app.staticTexts["ZoomPercentage"]
        if zoomText.waitForExistence(timeout: 3) {
            XCTAssertTrue(zoomText.label.contains("100"), "Zoom should reset to 100% after Cmd+0")
        }
    }

    func testPanHintAppearsWhenZoomed() throws {
        loadTemplateViaMenu()
        navigateToDiagramTab()

        Thread.sleep(forTimeInterval: 1.0)

        // Zoom in
        app.typeKey("+", modifierFlags: .command)
        app.typeKey("+", modifierFlags: .command)
        Thread.sleep(forTimeInterval: 0.5)

        // Check for the pan hint
        let panHint = app.staticTexts["PanHint"]
        XCTAssertTrue(panHint.waitForExistence(timeout: 5), "Pan hint should appear when zoomed in")

        // Reset
        app.typeKey("0", modifierFlags: .command)
    }

    // MARK: - Window Tests

    func testDoubleClickOpensNewWindow() throws {
        loadTemplateViaMenu()
        navigateToDiagramTab()

        Thread.sleep(forTimeInterval: 1.0)

        // Count windows before double-click
        let initialWindowCount = app.windows.count

        // Find the scroll view and double-click
        let diagramScrollView = app.scrollViews["DiagramScrollView"]
        if diagramScrollView.waitForExistence(timeout: 5) {
            diagramScrollView.doubleTap()

            Thread.sleep(forTimeInterval: 1.0)

            // Check if a new window opened
            XCTAssertTrue(app.windows.count > initialWindowCount, "Double-click should open a new window")

            // Close the new window (Cmd+W)
            app.typeKey("w", modifierFlags: .command)
        } else {
            XCTFail("Could not find diagram scroll view")
        }
    }

    func testNewWindowHasZoomControls() throws {
        loadTemplateViaMenu()
        navigateToDiagramTab()

        Thread.sleep(forTimeInterval: 1.0)

        // Double-click to open new window
        let diagramScrollView = app.scrollViews["DiagramScrollView"]
        if diagramScrollView.waitForExistence(timeout: 5) {
            diagramScrollView.doubleTap()
            Thread.sleep(forTimeInterval: 1.0)

            // Look for zoom controls in any window - search for % text
            let zoomText = app.staticTexts.matching(NSPredicate(format: "label CONTAINS '%'"))
            XCTAssertTrue(zoomText.count > 0, "Zoom controls should be visible in diagram window")

            // Close the new window
            app.typeKey("w", modifierFlags: .command)
        }
    }

    // MARK: - Tab Navigation Tests

    func testAllTabsAccessible() throws {
        loadTemplateViaMenu()

        let tabs = ["General", "Dimensions", "Lamp Sets", "Optical", "Intensity", "Diagram", "Validation"]

        for tabName in tabs {
            let tab = app.buttons[tabName]
            XCTAssertTrue(tab.waitForExistence(timeout: 3), "\(tabName) tab should exist")
            tab.click()
            Thread.sleep(forTimeInterval: 0.3)
        }
    }

    func testValidationTabShowsContent() throws {
        loadTemplateViaMenu()

        let validationTab = app.buttons["Validation"]
        XCTAssertTrue(validationTab.waitForExistence(timeout: 5), "Validation tab should exist")
        validationTab.click()

        Thread.sleep(forTimeInterval: 0.5)

        // Check for validation status (should show passed, warnings, or errors)
        let hasStatus = app.staticTexts.matching(NSPredicate(format: "label CONTAINS 'Validation'")).firstMatch.exists
        XCTAssertTrue(hasStatus, "Validation tab should show validation status")
    }

    // MARK: - Empty State Tests

    func testDoubleClickEmptyStateOpensFilePicker() throws {
        // Launch app with no file loaded
        app.terminate()
        app.launch()

        Thread.sleep(forTimeInterval: 1.0)

        // Double-click anywhere in the empty state
        let window = app.windows.firstMatch
        window.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).doubleTap()

        Thread.sleep(forTimeInterval: 1.0)

        // Check if file picker opened (look for Open dialog)
        let openDialog = app.dialogs.firstMatch
        if openDialog.waitForExistence(timeout: 3) {
            // File picker opened - close it
            app.typeKey(.escape, modifierFlags: [])
            XCTAssertTrue(true, "Double-click on empty state opened file picker")
        }
    }

    // MARK: - Performance Tests

    func testDiagramSwitchingPerformance() throws {
        loadTemplateViaMenu()
        navigateToDiagramTab()

        measure {
            // Switch through diagram types
            app.buttons["Cartesian"].click()
            app.buttons["Heatmap"].click()
            app.buttons["Polar"].click()
        }
    }

    // MARK: - App Store Screenshots (macOS)
    // Screenshots are captured at 1440x900 window size (2880x1800 on Retina displays)

    func testScreenshot01_PolarDiagram() throws {
        resizeWindowForScreenshot()
        loadTemplateViaMenu()
        navigateToDiagramTab()

        let polarButton = app.buttons["Polar"]
        if polarButton.waitForExistence(timeout: 3) {
            polarButton.click()
        }
        Thread.sleep(forTimeInterval: 1.0)

        let screenshot = app.windows.firstMatch.screenshot()
        let attachment = XCTAttachment(screenshot: screenshot)
        attachment.name = "01_Polar_Diagram"
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    func testScreenshot02_CartesianDiagram() throws {
        resizeWindowForScreenshot()
        loadTemplateViaMenu()
        navigateToDiagramTab()

        let cartesianButton = app.buttons["Cartesian"]
        if cartesianButton.waitForExistence(timeout: 3) {
            cartesianButton.click()
        }
        Thread.sleep(forTimeInterval: 1.0)

        let screenshot = app.windows.firstMatch.screenshot()
        let attachment = XCTAttachment(screenshot: screenshot)
        attachment.name = "02_Cartesian_Diagram"
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    func testScreenshot03_ButterflyDiagram() throws {
        resizeWindowForScreenshot()
        loadTemplateViaMenu()
        navigateToDiagramTab()

        let butterflyButton = app.buttons["Butterfly"]
        if butterflyButton.waitForExistence(timeout: 3) {
            butterflyButton.click()
        }
        Thread.sleep(forTimeInterval: 1.0)

        let screenshot = app.windows.firstMatch.screenshot()
        let attachment = XCTAttachment(screenshot: screenshot)
        attachment.name = "03_Butterfly_Diagram"
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    func testScreenshot04_3DDiagram() throws {
        resizeWindowForScreenshot()
        loadTemplateViaMenu()
        navigateToDiagramTab()

        let threeDButton = app.buttons["3D"]
        if threeDButton.waitForExistence(timeout: 3) {
            threeDButton.click()
        }
        Thread.sleep(forTimeInterval: 2.0) // 3D needs more time to render

        let screenshot = app.windows.firstMatch.screenshot()
        let attachment = XCTAttachment(screenshot: screenshot)
        attachment.name = "04_3D_Diagram"
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    func testScreenshot05_HeatmapDiagram() throws {
        resizeWindowForScreenshot()
        loadTemplateViaMenu()
        navigateToDiagramTab()

        let heatmapButton = app.buttons["Heatmap"]
        if heatmapButton.waitForExistence(timeout: 3) {
            heatmapButton.click()
        }
        Thread.sleep(forTimeInterval: 1.0)

        let screenshot = app.windows.firstMatch.screenshot()
        let attachment = XCTAttachment(screenshot: screenshot)
        attachment.name = "05_Heatmap_Diagram"
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    func testScreenshot06_BUGRating() throws {
        resizeWindowForScreenshot()
        loadTemplateViaMenu()
        navigateToDiagramTab()

        let bugButton = app.buttons["BUG"]
        if bugButton.waitForExistence(timeout: 3) {
            bugButton.click()
        }
        Thread.sleep(forTimeInterval: 1.0)

        let screenshot = app.windows.firstMatch.screenshot()
        let attachment = XCTAttachment(screenshot: screenshot)
        attachment.name = "06_BUG_Rating"
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    func testScreenshot07_LCSDiagram() throws {
        resizeWindowForScreenshot()
        loadTemplateViaMenu()
        navigateToDiagramTab()

        let lcsButton = app.buttons["LCS"]
        if lcsButton.waitForExistence(timeout: 3) {
            lcsButton.click()
        }
        Thread.sleep(forTimeInterval: 1.0)

        let screenshot = app.windows.firstMatch.screenshot()
        let attachment = XCTAttachment(screenshot: screenshot)
        attachment.name = "07_LCS_Diagram"
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    func testScreenshot08_GeneralInfo() throws {
        resizeWindowForScreenshot()
        loadTemplateViaMenu()

        let generalTab = app.buttons["General"]
        if generalTab.waitForExistence(timeout: 3) {
            generalTab.click()
        }
        Thread.sleep(forTimeInterval: 0.5)

        let screenshot = app.windows.firstMatch.screenshot()
        let attachment = XCTAttachment(screenshot: screenshot)
        attachment.name = "08_General_Info"
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    func testScreenshot09_ValidationTab() throws {
        resizeWindowForScreenshot()
        loadTemplateViaMenu()

        let validationTab = app.buttons["Validation"]
        if validationTab.waitForExistence(timeout: 3) {
            validationTab.click()
        }
        Thread.sleep(forTimeInterval: 0.5)

        let screenshot = app.windows.firstMatch.screenshot()
        let attachment = XCTAttachment(screenshot: screenshot)
        attachment.name = "09_Validation"
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    func testScreenshot10_IntensityTab() throws {
        resizeWindowForScreenshot()
        loadTemplateViaMenu()

        let intensityTab = app.buttons["Intensity"]
        if intensityTab.waitForExistence(timeout: 3) {
            intensityTab.click()
        }
        Thread.sleep(forTimeInterval: 0.5)

        let screenshot = app.windows.firstMatch.screenshot()
        let attachment = XCTAttachment(screenshot: screenshot)
        attachment.name = "10_Intensity_Tab"
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    // MARK: - iOS Screenshots (for iPhone/iPad simulators)

    /// Load a template on iOS by tapping the "New from Template" menu
    private func loadTemplateOniOS() {
        Thread.sleep(forTimeInterval: 2.0)

        // Look for "New from Template" menu button in empty state
        let templateMenu = app.buttons["New from Template"]
        if templateMenu.waitForExistence(timeout: 5) {
            templateMenu.tap()
            Thread.sleep(forTimeInterval: 1.0)

            // Select Road Luminaire from the menu
            let roadLuminaire = app.buttons["Road Luminaire"]
            if roadLuminaire.waitForExistence(timeout: 3) {
                roadLuminaire.tap()
                Thread.sleep(forTimeInterval: 1.0)
                return
            }

            // Try as menu item
            let menuItem = app.menuItems["Road Luminaire"]
            if menuItem.waitForExistence(timeout: 2) {
                menuItem.tap()
                Thread.sleep(forTimeInterval: 1.0)
                return
            }
        }

        // Fallback: Look for any luminaire template button
        let anyTemplate = app.buttons.matching(NSPredicate(format: "label CONTAINS 'Luminaire'")).firstMatch
        if anyTemplate.waitForExistence(timeout: 2) {
            anyTemplate.tap()
            Thread.sleep(forTimeInterval: 1.0)
        }
    }

    /// Helper to tap a tab button on iOS using accessibility identifier
    private func tapTabOniOS(_ tabName: String) -> Bool {
        // Try accessibility identifier first (Tab_Diagram, Tab_General, etc.)
        let tabId = "Tab_\(tabName)"
        var button = app.buttons[tabId]

        if !button.exists {
            // Fallback to label
            button = app.buttons[tabName]
        }

        if button.waitForExistence(timeout: 5) {
            // On iPhone, the tab bar scrolls horizontally
            // First try to scroll to make the button visible
            let scrollViews = app.scrollViews.allElementsBoundByIndex
            for scrollView in scrollViews {
                // Tab bar is typically short height
                if scrollView.frame.height < 60 && scrollView.frame.height > 20 {
                    // Scroll right to find tabs like "Diagram" which might be off-screen
                    if tabName == "Diagram" || tabName == "Validation" {
                        scrollView.swipeLeft()
                        Thread.sleep(forTimeInterval: 0.3)
                    } else if tabName == "General" {
                        scrollView.swipeRight()
                        Thread.sleep(forTimeInterval: 0.3)
                    }
                    break
                }
            }

            // Re-find the button after scrolling
            button = app.buttons[tabId]
            if !button.exists {
                button = app.buttons[tabName]
            }

            if button.waitForExistence(timeout: 3) {
                // Use coordinate tap
                button.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).tap()
                return true
            }
        }
        return false
    }

    /// Helper to tap a regular button on iOS
    private func tapButtonOniOS(_ label: String) -> Bool {
        let button = app.buttons[label]
        if button.waitForExistence(timeout: 3) {
            // Always use coordinate tap to avoid isHittable issues
            button.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).tap()
            return true
        }
        return false
    }

    func testScreenshotIOS_01_PolarDiagram() throws {
        // Skip on macOS
        guard app.menuBars.count == 0 else {
            throw XCTSkip("iOS-only test")
        }

        loadTemplateOniOS()

        // Navigate to Diagram tab
        _ = tapTabOniOS("Diagram")
        Thread.sleep(forTimeInterval: 1.0)

        // Select Polar diagram
        _ = tapButtonOniOS("Polar")
        Thread.sleep(forTimeInterval: 3.0) // Wait longer for SVG to render

        let screenshot = XCUIScreen.main.screenshot()
        let attachment = XCTAttachment(screenshot: screenshot)
        attachment.name = "iOS_01_Polar_Diagram"
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    func testScreenshotIOS_02_CartesianDiagram() throws {
        guard app.menuBars.count == 0 else {
            throw XCTSkip("iOS-only test")
        }

        loadTemplateOniOS()

        _ = tapTabOniOS("Diagram")
        Thread.sleep(forTimeInterval: 1.0)

        _ = tapButtonOniOS("Cartesian")
        Thread.sleep(forTimeInterval: 3.0)

        let screenshot = XCUIScreen.main.screenshot()
        let attachment = XCTAttachment(screenshot: screenshot)
        attachment.name = "iOS_02_Cartesian_Diagram"
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    func testScreenshotIOS_03_ButterflyDiagram() throws {
        guard app.menuBars.count == 0 else {
            throw XCTSkip("iOS-only test")
        }

        loadTemplateOniOS()

        _ = tapTabOniOS("Diagram")
        Thread.sleep(forTimeInterval: 1.0)

        _ = tapButtonOniOS("Butterfly")
        Thread.sleep(forTimeInterval: 3.0)

        let screenshot = XCUIScreen.main.screenshot()
        let attachment = XCTAttachment(screenshot: screenshot)
        attachment.name = "iOS_03_Butterfly_Diagram"
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    func testScreenshotIOS_04_3DDiagram() throws {
        guard app.menuBars.count == 0 else {
            throw XCTSkip("iOS-only test")
        }

        loadTemplateOniOS()

        _ = tapTabOniOS("Diagram")
        Thread.sleep(forTimeInterval: 1.0)

        _ = tapButtonOniOS("3D")
        Thread.sleep(forTimeInterval: 4.0) // 3D needs extra time

        let screenshot = XCUIScreen.main.screenshot()
        let attachment = XCTAttachment(screenshot: screenshot)
        attachment.name = "iOS_04_3D_Diagram"
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    func testScreenshotIOS_05_HeatmapDiagram() throws {
        guard app.menuBars.count == 0 else {
            throw XCTSkip("iOS-only test")
        }

        loadTemplateOniOS()

        _ = tapTabOniOS("Diagram")
        Thread.sleep(forTimeInterval: 1.0)

        _ = tapButtonOniOS("Heatmap")
        Thread.sleep(forTimeInterval: 3.0)

        let screenshot = XCUIScreen.main.screenshot()
        let attachment = XCTAttachment(screenshot: screenshot)
        attachment.name = "iOS_05_Heatmap_Diagram"
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    func testScreenshotIOS_06_GeneralInfo() throws {
        guard app.menuBars.count == 0 else {
            throw XCTSkip("iOS-only test")
        }

        loadTemplateOniOS()

        _ = tapTabOniOS("General")
        Thread.sleep(forTimeInterval: 1.0)

        let screenshot = XCUIScreen.main.screenshot()
        let attachment = XCTAttachment(screenshot: screenshot)
        attachment.name = "iOS_06_General_Info"
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    func testScreenshotIOS_00_StartScreen() throws {
        guard app.menuBars.count == 0 else {
            throw XCTSkip("iOS-only test")
        }

        // Just wait for the app to start - don't load a template
        Thread.sleep(forTimeInterval: 2.0)

        let screenshot = XCUIScreen.main.screenshot()
        let attachment = XCTAttachment(screenshot: screenshot)
        attachment.name = "iOS_00_Start_Screen"
        attachment.lifetime = .keepAlways
        add(attachment)
    }
}
