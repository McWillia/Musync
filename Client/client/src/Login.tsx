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
    const redirect_uri = "http://localhost:3000/home";
    const scopes = [
      "user-read-currently-playing",
      "user-read-playback-state",
    ];
    return (
      <div className="App">
        <header className="App-header">
          <a href={'https://accounts.spotify.com/authorize' +
          '?response_type=code' +
          '&client_id=' + my_client_id +
          (scopes ? '&scope=' + encodeURIComponent(scopes.join(" ")) : '') +
          '&redirect_uri=' + encodeURIComponent(redirect_uri)}>
            Textual
          </a>
        </header>
      </div>
    );
  }

}

export default App;
