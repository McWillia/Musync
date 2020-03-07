import React from 'react';
import logo from './logo.svg';
import './Login.css';

export const authEndpoint = 'https://accounts.spotify.com/authorize';

const clientId = "f092792439d74b7e9341f90719b98365";
const redirectUri = "http://localhost:3000";
const scopes = [
  "user-read-currently-playing",
  "user-read-playback-state",
];

const hash = window.location.hash
  .substring(1)
  .split("&")
  .reduce(function(initial, item) {
    if (item) {
      var parts = item.split("=");
      initial[parts[0]] = decodeURIComponent(parts[1]);
    }
    return initial;
  }, {});

  window.location.hash = "";

class App extends React.Component {

  constructor (props){
    super(props);
    this.state = {
      token: null
    }
    this.handleClick = this.handleClick.bind(this);
  }

  componentDidMount() {
    // Set token
    // let _token = hash.access_token;
    // if (_token) {
    //   // Set token
    //   this.setState({
    //     token: _token
    //   });
    // }

    fetch("http://localhost:8080", {
      method: 'GET',
      mode: 'cors',
      headers:{
        'Content-Type': 'application/json',
        'Access-Control-Allow-Origin':'*'
      },
    })
  }

  handleClick(){
    const my_client_id="f092792439d74b7e9341f90719b98365";
    const redirect_uri = "http://localhost:8080/home"
    fetch('https://accounts.spotify.com/authorize' +
    '?response_type=code' +
    '&client_id=' + my_client_id +
    (scopes ? '&scope=' + encodeURIComponent(scopes) : '') +
    '&redirect_uri=' + encodeURIComponent(redirect_uri), {
      method: 'GET',
      mode: 'cors',
      headers:{
        'Content-Type': 'application/json',

      },
    })
    .then(res => res.json())
    .then(out => console.log(out))
    .catch(error => console.log(error))
  }
  render(){
    const my_client_id="f092792439d74b7e9341f90719b98365";
    const redirect_uri = "http://localhost:3000/home"
    return (
      <div className="App">
        <header className="App-header">
          {/* <input type="button" onClick = {this.handleClick}/> */}
          <a href={'https://accounts.spotify.com/authorize' +
    '?response_type=code' +
    '&client_id=' + my_client_id +
    (scopes ? '&scope=' + encodeURIComponent(scopes) : '') +
    '&redirect_uri=' + encodeURIComponent(redirect_uri)}>
          Textual

        </a>



          </header>
      </div>
    );
  }

}

export default App;
