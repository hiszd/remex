**Notes**
the intended function of the server is to be the part the user interacts with.
It will need to do the following:
- [ ] Handle connections with clients
- [ ] Handle messages from clients
  - [ ] Messages from the client will only contain information, not commands, so they need to be parsed and logged
  - [ ] Messages will need to communicate what application, or operation they pertain to so that the log can be filtered as such
- Issue commands to the clients
  - [ ] If it is, execute the command
  - [ ] Receive the log from the command after it's execution
- Use either named pipes(Windows) or unix sockets(Linux) to allow the server to communicate with a GUI of some kind
  - [ ] The GUI will be able to add, or manage commands, and applications that the server requires to be executed on the client
  - [ ] The GUI can view the logs of each command, and see if it was successful or not upon execution
  - [ ] The GUI will show exectly what clients match the filter for a command