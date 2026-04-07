# AGENTS.md

## Directory Structure

The directory and project are laid out as such:
'/core' - The shared library for all remex executables
'/server' - The server executable and it's related source code
'/endpoint' - The endpoint executable and it's related source code
'/configurator' - The configurator Vue.js web application

## Systems Design

This system utilizes a client-server architecture at it's core.

remex_core is a shared library that holds all shared logic for the server and the endpoint.

remex_server houses the application's central server which maintains connections via an encrypted TCP socket. It connects to the core SurrealDB database. The messages sent over the TCP socket from the server aren't for the purpose of sending database table queries to the endpoints, but instead is a centralized method of pointing the endpoints to the core database in the cloud.

remex_endpoint is the edge client that both connects with the remex_server and connects to the core SurrealDB database in the cloud. In addition to this, it also manages a local database for caching updates to be sent to the core database, and to reference for jobs that may need to be executed offline. It spawns several background tasks that monitor things like whether a job should be run yet, whether to respond to a server message, etc.

remex_configurator is a standalone Vue.js web application for the end user to create new configurations, modify existing ones, and check on the execution status of each job. It connects directly to its own SurrealDB database (separate from the core database used by endpoints).

## CSS/SCSS styles

All styles should use colors from the themes, and not hard-coded colors. If a theme color needs to be added, it should be added to the theme file.
