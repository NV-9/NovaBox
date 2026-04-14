package net.novabox.velocity;

import com.google.gson.Gson;
import com.google.gson.JsonObject;
import com.velocitypowered.api.proxy.ProxyServer;
import com.velocitypowered.api.proxy.server.RegisteredServer;
import com.velocitypowered.api.proxy.server.ServerInfo;
import org.slf4j.Logger;

import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.net.ServerSocket;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.util.Optional;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

public class HttpApiServer {

    private final ProxyServer proxy;
    private final Logger logger;
    private final int port;
    private final String secret;
    private final Gson gson = new Gson();

    private ServerSocket serverSocket;
    private final ExecutorService executor = Executors.newCachedThreadPool();
    private volatile boolean running = false;

    public HttpApiServer(ProxyServer proxy, Logger logger, int port, String secret) {
        this.proxy = proxy;
        this.logger = logger;
        this.port = port;
        this.secret = secret;
    }

    public void start() throws IOException {
        serverSocket = new ServerSocket(port);
        running = true;
        executor.submit(() -> {
            while (running) {
                try {
                    Socket client = serverSocket.accept();
                    executor.submit(() -> handleClient(client));
                } catch (IOException e) {
                    if (running) {
                        logger.error("Accept error", e);
                    }
                }
            }
        });
    }

    public void stop() {
        running = false;
        try {
            if (serverSocket != null) serverSocket.close();
        } catch (IOException ignored) {}
        executor.shutdownNow();
    }

    private void handleClient(Socket client) {
        try (client) {
            InputStream in = client.getInputStream();
            OutputStream out = client.getOutputStream();

            byte[] buf = new byte[8192];
            int n = in.read(buf);
            if (n <= 0) return;

            String raw = new String(buf, 0, n, StandardCharsets.UTF_8);
            String[] lines = raw.split("\r\n");
            if (lines.length == 0) return;

            String[] requestLine = lines[0].split(" ");
            if (requestLine.length < 2) return;

            String method = requestLine[0];
            String path   = requestLine[1];

            if (!secret.isEmpty()) {
                String authHeader = null;
                for (String line : lines) {
                    if (line.toLowerCase().startsWith("x-novabox-secret:")) {
                        authHeader = line.substring(line.indexOf(':') + 1).trim();
                        break;
                    }
                }
                if (!secret.equals(authHeader)) {
                    write(out, 403, "Forbidden");
                    return;
                }
            }

            String body = "";
            int bodyStart = raw.indexOf("\r\n\r\n");
            if (bodyStart != -1) {
                body = raw.substring(bodyStart + 4);
            }

            if (method.equals("GET") && path.equals("/health")) {
                write(out, 200, "{\"status\":\"ok\"}");

            } else if (method.equals("POST") && path.equals("/servers")) {
                handleRegister(out, body);

            } else if (method.equals("DELETE") && path.startsWith("/servers/")) {
                String name = path.substring("/servers/".length());
                handleUnregister(out, name);

            } else if (method.equals("GET") && path.equals("/servers")) {
                handleList(out);

            } else {
                write(out, 404, "Not Found");
            }

        } catch (Exception e) {
            logger.error("Error handling client", e);
        }
    }

    private void handleRegister(OutputStream out, String body) throws IOException {
        JsonObject json;
        try {
            json = gson.fromJson(body, JsonObject.class);
        } catch (Exception e) {
            write(out, 400, "Invalid JSON");
            return;
        }

        if (!json.has("name") || !json.has("host") || !json.has("port")) {
            write(out, 400, "Missing required fields: name, host, port");
            return;
        }

        String name = json.get("name").getAsString();
        String host = json.get("host").getAsString();
        int    port = json.get("port").getAsInt();

        ServerInfo info = new ServerInfo(name, new InetSocketAddress(host, port));

        Optional<RegisteredServer> existing = proxy.getServer(name);
        if (existing.isPresent()) {
            proxy.unregisterServer(existing.get().getServerInfo());
        }

        proxy.registerServer(info);
        logger.info("Registered server: {} -> {}:{}", name, host, port);
        write(out, 200, "{\"registered\":true,\"name\":\"" + name + "\"}");
    }

    private void handleUnregister(OutputStream out, String name) throws IOException {
        Optional<RegisteredServer> server = proxy.getServer(name);
        if (server.isEmpty()) {
            write(out, 404, "Server not found: " + name);
            return;
        }
        proxy.unregisterServer(server.get().getServerInfo());
        logger.info("Unregistered server: {}", name);
        write(out, 200, "{\"unregistered\":true,\"name\":\"" + name + "\"}");
    }

    private void handleList(OutputStream out) throws IOException {
        StringBuilder sb = new StringBuilder("[");
        boolean first = true;
        for (RegisteredServer s : proxy.getAllServers()) {
            if (!first) sb.append(",");
            first = false;
            sb.append("{\"name\":\"").append(s.getServerInfo().getName()).append("\"")
              .append(",\"host\":\"").append(s.getServerInfo().getAddress().getHostString()).append("\"")
              .append(",\"port\":").append(s.getServerInfo().getAddress().getPort())
              .append("}");
        }
        sb.append("]");
        write(out, 200, sb.toString());
    }

    private void write(OutputStream out, int status, String body) throws IOException {
        String reason = status == 200 ? "OK" : status == 400 ? "Bad Request" : status == 403 ? "Forbidden" : status == 404 ? "Not Found" : "Error";
        String response = "HTTP/1.1 " + status + " " + reason + "\r\n"
            + "Content-Type: application/json\r\n"
            + "Content-Length: " + body.getBytes(StandardCharsets.UTF_8).length + "\r\n"
            + "Connection: close\r\n"
            + "\r\n"
            + body;
        out.write(response.getBytes(StandardCharsets.UTF_8));
        out.flush();
    }
}
