const express = require('express');
const bodyParser = require('body-parser');
const cors = require('cors');
const WebSocket = require("ws");
const fetch = require('node-fetch');


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

const redirectUri = "http://localhost:3000/home";

const secrets = Buffer.from('f092792439d74b7e9341f90719b98365:3b2f3bf79fc14c10967dca3dc97aacaf').toString('base64');

let users = new Map();

server.on('connection', function connection(ws) {
    console.log("Connection established");
    let code;
    ws.on('message', function incoming(message) {
        var msg = JSON.parse(message);

        console.log("Message from client: " + msg);
        console.log(msg);

        switch (msg.type) {
            case 'authCode':
                console.log("!!!");

                var body = {
                    grant_type:'authorization_code',
                    code:msg.code,
                    redirect_uri:redirectUri
                }

                fetch('https://accounts.spotify.com/api/token',
                {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/x-www-form-urlencoded',
                        'Authorization': 'Basic ' + secrets
                    },
                    body: new URLSearchParams(body)

                })
                // .then(this.handleErrors)
                .then((response) => response.json())
                .then((data) =>{
                    console.log("Success: ");
                    console.log(data);

                    users.set(msg.code, {
                        'groupID': 0,
                        'token': data,
                        'ws':ws
                    })
                })
                .catch((error) =>{
                    console.log("Get errored:" + error);
                });
                break;
            default:
                console.log("Unknown message type");
        }

    });

    ws.on('close', function closing(code, reason){
        if (code) {
            users.delete(code);
        }
    })
})







app.use(function (req, res, next){
    res.status(404).send("Server does not send Files. Please try localhost:3000");
});

app.listen(http_port, function() {
    console.log("Listening for HTTP on localhost:" + http_port);
});
