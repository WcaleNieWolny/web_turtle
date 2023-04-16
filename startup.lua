local ws, err = http.websocket("ws://127.0.0.1:8080/turtle/")
if err then
  print(err)
else if ws then
  print("> CONNECTED")
  while true do
    local message = ws.receive()
	local output = load(message)()
	ws.send(output)
  end
end
end
