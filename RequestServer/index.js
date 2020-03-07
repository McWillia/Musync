const express = require('express');
const bodyParser = require('body-parser');
const cors = require('cors');
const WebSocket = require("ws");
const fetch = require('node-fetch');
const os = require('os');

const networkInterfaces = os.networkInterfaces();
const host = networkInterfaces.enp3s0[0].address;
console.log(host);

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
const microservices = new WebSocket.Server({port:8082});

const redirectUri = "http://localhost:3000/home";
const baseAPI = 'https://api.spotify.com';


const secrets = Buffer.from('f092792439d74b7e9341f90719b98365:3b2f3bf79fc14c10967dca3dc97aacaf').toString('base64');

let users = new Map();
let groups = new Map();
let services = new Map();
let groupNumber = 0;


server.on('connection', function connection(ws) {
    console.log("Connection established");
    let code;

    function updateGroups (){
        for (user of users.values()) {
            user.ws.send(JSON.stringify({
                'type': 'advertising_groups',
                'data': [...groups.values()]
            }));
        }
    }

    ws.on('message', function incoming(message) {
        var msg = JSON.parse(message);

        console.log("Message from client: " + msg);
        // console.log(msg);

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

                    //Add user to new group and place in user map
                    users.set(msg.code, {
                        'groupID': groupNumber,
                        'token': data,
                        'ws':ws
                    })
                    var clientArray = [];
                    clientArray.push(msg.code);
                    //Make new group with only new client
                    groups.set(groupNumber, {
                        'advert': false,
                        'id': groupNumber,
                        'clients':clientArray
                    })

                    code = msg.code;

                    updateGroups()

                    console.log(groups)

                    groupNumber++;
                })
                .catch((error) =>{
                    console.log("Get errored:" + error);
                });
                break;

            case 'get_playlists':

                fetch(baseAPI + '/v1/me/playlists',
                {
                    method: 'GET',
                    headers: {
                        'Authorization': 'Bearer ' + users.get(msg.code).token.access_token
                    },

                })
                // .then(this.handleErrors)
                .then((response) => response.json())
                .then((data) =>{
                    console.log("Success: ");
                    console.log(data);
                    ws.send(JSON.stringify({
                        'type':'response_playlists',
                        'data': data
                    }));
                })
                .catch((error) =>{
                    console.log("Get errored:" + error);
                });


                break;

            case 'make_mutual_playlist':
                /*
                Get a mutual playlist socket
                Get group clients
                Send clients to socket
                */
                if (services.get('MutualPlaylist')) {
                    let access_token = users.get(msg.code).token.access_token;
                    services.get('MutualPlaylist').send(JSON.stringify({
                        access_token: access_token
                    }))
                }
                break;

            // case 'get_advertising_groups':
            //     console.log("ADDDDD");
            //     console.log([...groups.values()]);
            //
            //     break;
            case 'join_group':
                // msg.id
                // msg.code

                if (groups.get(msg.id)) {
                    //Get the old group
                    let group = groups.get(users.get(msg.code).groupID);
                    //remove from old group
                    group.clients = group.clients.filter((client) => client != code)
                    //Check if group is empty
                    if (group.clients.length === 0) {
                        console.log("Removing group" + users.get(code).groupID);
                        groups.delete(users.get(code).groupID)
                    }

                    //Update the map
                    users.get(msg.code).groupID = msg.id;
                    //Add to new group
                    groups.get(msg.id).clients.push(msg.code);


                    updateGroups();
                }
                break;
            default:
                console.log("Unknown message type");
        }

    });

    ws.on('close', function closing(code_error, reason){
        console.log("CLOSING");
        console.log(code);
        if (code) {
            console.log("-------------------");
            console.log(users.get(code));
            console.log("-------------------");
            console.log(users)
            console.log("-------------------");
            console.log(code);
            console.log("-------------------");
            let group = groups.get(users.get(code).groupID);

            group.clients = group.clients.filter((client) => client != code)

            if (group.clients.length === 0) {
                console.log("Here");
                groups.delete(users.get(code).groupID)
            }

            users.delete(code);

            updateGroups();

        }
    })
})



microservices.on('connection', function connection(ws){
    ws.on('message', function incoming(message) {
        var msg = JSON.parse(message);

        switch (msg.type) {
            case 'new':
                console.log(msg.microservice_type);
                services.set(msg.microservice_type, ws)
                break;
            case 'result':

                break;
            default:

        }
    })
})




app.use(function (req, res, next){
    res.status(404).send("Server does not send Files. Please try localhost:3000");
});

app.listen(http_port, function() {
    console.log("Listening for HTTP on localhost:" + http_port);
});
