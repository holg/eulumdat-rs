import SwiftUI

#if os(macOS)
import AppKit

struct SVGView: NSViewRepresentable {
    let svgString: String

    func makeNSView(context: Context) -> NSView {
        let view = SVGNSView()
        view.svgString = svgString
        return view
    }

    func updateNSView(_ nsView: NSView, context: Context) {
        if let svgView = nsView as? SVGNSView {
            svgView.svgString = svgString
            svgView.needsDisplay = true
        }
    }
}

class SVGNSView: NSView {
    var svgString: String = "" {
        didSet {
            cachedImage = nil
            needsDisplay = true
        }
    }
    private var cachedImage: NSImage?

    override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)

        guard !svgString.isEmpty else { return }

        if cachedImage == nil {
            guard let data = svgString.data(using: .utf8) else { return }
            cachedImage = NSImage(data: data)
        }

        cachedImage?.draw(in: bounds)
    }
}

#else
import UIKit
import WebKit

struct SVGView: UIViewRepresentable {
    let svgString: String

    func makeUIView(context: Context) -> WKWebView {
        let config = WKWebViewConfiguration()
        config.defaultWebpagePreferences.allowsContentJavaScript = false

        let webView = WKWebView(frame: .zero, configuration: config)
        webView.isOpaque = false
        webView.backgroundColor = .clear
        webView.scrollView.backgroundColor = .clear
        webView.scrollView.isScrollEnabled = false
        webView.scrollView.bounces = false
        return webView
    }

    func updateUIView(_ webView: WKWebView, context: Context) {
        // Ensure SVG fills the container properly
        let html = """
        <!DOCTYPE html>
        <html>
        <head>
            <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
            <style>
                * {
                    margin: 0;
                    padding: 0;
                    box-sizing: border-box;
                }
                html, body {
                    width: 100%;
                    height: 100%;
                    overflow: hidden;
                    background: transparent;
                }
                body {
                    display: flex;
                    justify-content: center;
                    align-items: center;
                }
                svg {
                    width: 100%;
                    height: 100%;
                    display: block;
                }
            </style>
        </head>
        <body>
            \(svgString)
        </body>
        </html>
        """
        webView.loadHTMLString(html, baseURL: nil)
    }
}
#endif
