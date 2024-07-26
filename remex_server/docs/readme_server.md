The purpose of this project is to create a remote execution protocol that has the following features:
- Detection methods including registry key, file exists, script, etc.
- Run as active user, or in system context
- Log all commands run, and their STDOUT

This part of the project is the server.
The job of the server in this context is to 
- hold the configurations
- connect to the clients
- synchronize with the clients
- retain logs of all the client activities