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
        let webView = WKWebView()
        webView.isOpaque = false
        webView.backgroundColor = .clear
        webView.scrollView.isScrollEnabled = false
        return webView
    }

    func updateUIView(_ webView: WKWebView, context: Context) {
        let html = """
        <!DOCTYPE html>
        <html>
        <head>
            <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
            <style>
                body {
                    margin: 0;
                    padding: 0;
                    display: flex;
                    justify-content: center;
                    align-items: center;
                    min-height: 100vh;
                    background: transparent;
                }
                svg {
                    max-width: 100%;
                    max-height: 100%;
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
