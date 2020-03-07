const express = require('express');
const bodyParser = require('body-parser');
const cors = require('cors');
const WebSocket = require("ws");


const app = express();
const http_port = 8080;

app.use(cors());
app.use(bodyParser.json()); // support json encoded bodies
app.use(bodyParser.urlencoded({ extended: true })); // support encoded bodies
app.options('*', cors());

app.get('/test', function(req, res) {
    res.send(JSON.stringify({"hi":"there"}));
})

const server = new WebSocket.Server({ port: 8081 });

server.on('connection', function connection(ws) {
    console.log("Connection established");
    console.log("Sending to server");
    ws.on('message', function incoming(message) {
        console.log("Message from client: " + message);
    });
})







app.use(function (req, res, next){
    res.status(404).send("Server does not send Files. Please try localhost:3000");
});

app.listen(http_port, function() {
    console.log("Listening for HTTP on localhost:" + http_port);
});
