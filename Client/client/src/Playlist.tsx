import React, { Component } from "react";

interface IProps {
    playlist_data: string | null,
    code: string,
    client: WebSocket
}

interface IState {

}

export default class Playlist extends Component<IProps, IState> {
    constructor(props: IProps) {
        super(props)
        this.sendRequest = this.sendRequest.bind(this);
    }
  sendRequest(){
      var {client, code} = this.props;
      // console.log(code);

    this.props.client.send(JSON.stringify({code: this.props.code, type: 'get_playlists'}));
  }

  render(){
    var obj;
    if(this.props.playlist_data !=null) {
      var data = JSON.parse(this.props.playlist_data);

      for(int i=0; i<data.length; i++){

      }
      obj = <div> content</div>

    } else {
      obj = <div></div>;
    }

    return(

      <div>
        <button onClick={this.sendRequest}  >test</button>

          {obj}

      </div>
    )
  }
}
