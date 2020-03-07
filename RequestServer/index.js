const express = require('express');
const bodyParser = require('body-parser');
const cors = require('cors');


const app = express();
const http_port = 8080;

app.use(cors());
app.use(bodyParser.json()); // support json encoded bodies
app.use(bodyParser.urlencoded({ extended: true })); // support encoded bodies
app.options('*', cors());

const my_client_id="f092792439d74b7e9341f90719b98365";
const redirect_uri = "http://localhost:8080/callback"

app.get('/login', function(req, res) {
var scopes = 'user-read-private user-read-email';
res.redirect('https://accounts.spotify.com/authorize' +
  '?response_type=code' +
  '&client_id=' + my_client_id +
  (scopes ? '&scope=' + encodeURIComponent(scopes) : '') +
  '&redirect_uri=' + encodeURIComponent(redirect_uri));
});



app.get('/callback', function(req, res){
    console.log(req);
    console.log(res);
})

app.get('/test', function(req, res) {
    res.send(JSON.stringify({"hi":"there"}));
})






app.use(function (req, res, next){
    res.status(404).send("Server does not send Files. Please try localhost:3000");
});

app.listen(http_port, function() {
    console.log("Listening for HTTP on localhost:" + http_port);
});
