**Notes**
the intended function of the client is to connect to the server and receive messages, as well as report back logging of executed commands
It will need to do the following:
- [ ] Handle connections with the server
- [ ] Handle messages from the server
  - [ ] Messages from the server will either mutate, remove, or create a command
- [ ] Execute the commands that the server sends down
  - [ ] Keep track of commands that have been executed and execute new commands if needed
  - [ ] Keep track of status of each command(e.g. pass, fail, running, etc.)