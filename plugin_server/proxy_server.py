#!/usr/bin/env python3
"""
Simple HTTP server with API proxy to work around CORS issues
"""
import http.server
import socketserver
import urllib.request
import urllib.parse
import json
import os

PORT = 8080
BACKEND_URL = "http://localhost:3000"

class ProxyHandler(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        if self.path.startswith('/api/'):
            self.proxy_request()
        else:
            super().do_GET()
    
    def do_POST(self):
        if self.path.startswith('/api/'):
            self.proxy_request()
        else:
            self.send_error(405, "Method not allowed")
    
    def do_PUT(self):
        if self.path.startswith('/api/'):
            self.proxy_request()
        else:
            self.send_error(405, "Method not allowed")
    
    def do_DELETE(self):
        if self.path.startswith('/api/'):
            self.proxy_request()
        else:
            self.send_error(405, "Method not allowed")
    
    def do_OPTIONS(self):
        if self.path.startswith('/api/'):
            self.send_response(200)
            self.send_header('Access-Control-Allow-Origin', '*')
            self.send_header('Access-Control-Allow-Methods', 'GET, POST, PUT, DELETE, OPTIONS')
            self.send_header('Access-Control-Allow-Headers', 'Content-Type, Authorization')
            self.end_headers()
        else:
            super().do_OPTIONS()
    
    def proxy_request(self):
        try:
            # Build the backend URL
            backend_url = BACKEND_URL + self.path
            
            # Prepare the request
            headers = {}
            for header, value in self.headers.items():
                if header.lower() not in ['host', 'connection']:
                    headers[header] = value
            
            # Get request body if it's a POST request
            content_length = int(self.headers.get('Content-Length', 0))
            post_data = None
            if content_length > 0:
                post_data = self.rfile.read(content_length)
            
            # Make the request to backend
            req = urllib.request.Request(backend_url, data=post_data, headers=headers, method=self.command)
            
            with urllib.request.urlopen(req) as response:
                # Send response
                self.send_response(response.status)
                
                # Copy headers
                for header, value in response.headers.items():
                    if header.lower() not in ['connection', 'transfer-encoding']:
                        self.send_header(header, value)
                
                # Add CORS headers
                self.send_header('Access-Control-Allow-Origin', '*')
                self.send_header('Access-Control-Allow-Methods', 'GET, POST, PUT, DELETE, OPTIONS')
                self.send_header('Access-Control-Allow-Headers', 'Content-Type, Authorization')
                
                self.end_headers()
                
                # Copy response body
                self.wfile.write(response.read())
                
        except Exception as e:
            print(f"Proxy error: {e}")
            self.send_error(500, f"Proxy error: {str(e)}")

if __name__ == "__main__":
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    
    with socketserver.TCPServer(("", PORT), ProxyHandler) as httpd:
        print(f"Serving at http://localhost:{PORT}")
        print(f"Proxying /api/* requests to {BACKEND_URL}")
        print("Press Ctrl+C to stop")
        httpd.serve_forever()