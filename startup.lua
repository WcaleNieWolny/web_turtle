local function unsafe_exec(func)
  local ok_e, err_e = func()
  if ok_e then
    return tostring(ok_e)
  else  
    return tostring(err_e)
  end
end  

local ws, err = http.websocket("ws://127.0.0.1:8080/turtle/")
if err then
  print(err)
else if ws then
  print("> CONNECTED")
  while true do
    local message = ws.receive()
    local func = load(message)
    local ok, retval = pcall(unsafe_exec, func)
    local result = tostring(retval)
    ws.send(result)
  end
end
end 
