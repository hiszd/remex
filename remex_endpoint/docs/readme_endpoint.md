The purpose of this project is to create a remote execution protocol that has the following features:
- Detection methods including registry key, file exists, script, etc.
- Run as active user, or in system context
- Log all commands run, and their STDOUT

This part of the project is the endpoint client.
The job of the endpoint in this context is to
- connect to the server
- synchronize configuration from server
- send logs for all the client activities