import React, { Component } from 'react';
import './Login.css';

interface IState {

}

interface IProps {

}

class App extends Component<IProps, IState> {

  constructor (props: IProps){
    super(props);
    this.state = {
      token: null
    }
  }

  render(){
    const my_client_id="f092792439d74b7e9341f90719b98365";
    const redirect_uri = "http://pc7-150-l:3000/home";
    const scopes = [
      "user-read-currently-playing",
      "user-read-playback-state",
      "playlist-read-private",
      "playlist-modify-private",
      "playlist-modify-public",
      "user-top-read",
      "user-modify-playback-state"
    ];
    return (
      <div className="App">
        <header className="App-header">
          <a href={'https://accounts.spotify.com/authorize' +
          '?response_type=code' +
          '&client_id=' + my_client_id +
          (scopes ? '&scope=' + encodeURIComponent(scopes.join(" ")) : '') +
          '&redirect_uri=' + encodeURIComponent(redirect_uri)}>
            Hello! Welcome to MuSinkc
          </a>
        </header>
      </div>
    );
  }

}

export default App;
