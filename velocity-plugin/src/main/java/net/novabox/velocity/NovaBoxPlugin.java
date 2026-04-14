package net.novabox.velocity;

import com.google.inject.Inject;
import com.velocitypowered.api.event.Subscribe;
import com.velocitypowered.api.event.proxy.ProxyInitializeEvent;
import com.velocitypowered.api.event.proxy.ProxyShutdownEvent;
import com.velocitypowered.api.plugin.Plugin;
import com.velocitypowered.api.proxy.ProxyServer;
import org.slf4j.Logger;

@Plugin(
    id = "novabox-velocity",
    name = "NovaBox Velocity",
    version = "1.0.0",
    description = "Dynamic server registration HTTP API for NovaBox",
    authors = {"NovaBox"}
)
public class NovaBoxPlugin {

    private final ProxyServer proxy;
    private final Logger logger;
    private HttpApiServer apiServer;

    @Inject
    public NovaBoxPlugin(ProxyServer proxy, Logger logger) {
        this.proxy = proxy;
        this.logger = logger;
    }

    @Subscribe
    public void onProxyInitialize(ProxyInitializeEvent event) {
        int port = Integer.parseInt(System.getenv().getOrDefault("NOVABOX_API_PORT", "7000"));
        String secret = System.getenv().getOrDefault("NOVABOX_API_SECRET", "");

        apiServer = new HttpApiServer(proxy, logger, port, secret);
        try {
            apiServer.start();
            logger.info("NovaBox API server listening on port {}", port);
        } catch (Exception e) {
            logger.error("Failed to start NovaBox API server", e);
        }
    }

    @Subscribe
    public void onProxyShutdown(ProxyShutdownEvent event) {
        if (apiServer != null) {
            apiServer.stop();
        }
    }
}
