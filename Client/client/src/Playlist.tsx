import React, { Component } from "react";

interface IProps {
    playlist_data?: string,
    code: string,
    client: WebSocket
}

interface IState {

}

interface I{

}

export default class Playlist extends Component<IProps, IState> {
    constructor(props: IProps) {
        super(props)
        this.sendRequest = this.sendRequest.bind(this);
    }
  sendRequest(){
      var {client, code} = this.props;

    this.props.client.send(JSON.stringify({code: this.props.code, type: 'get_playlists'}));
  }

  openPlaylist(song_name: string){
    alert("You have opened" + song_name)
  }

  render(){
    var obj = [];

    if(this.props.playlist_data !=null) {

      var data = this.props.playlist_data;
      // console.log(data);
      if (data) {

          var actual_data = JSON.parse(data);
          // console.log(actual_data);
          //
          //
          if(actual_data){
              // console.log(actual_data.items);

              obj = actual_data.items.map((playlist: any) => {
                return(
                  <div>
                    {playlist.name}
                    <button onClick={() => {this.openPlaylist(playlist.name)}}>Open {playlist.name}</button>
                    <br />
                  </div>
                )
              })

              // for(var i = 0; i < actual_data.items.length && actual_data.items; i++){
              //       obj.push(<div>{actual_data.items[i].name} <button onClick={() => {this.openPlaylist(actual_data.items[i].name)}}>Open {actual_data.items[i].name}</button><br /></div>);
              //     }
              }
      }

    } else{
      obj.push(<div></div>)
    }

    return(

      <div>
        <button onClick={this.sendRequest}  >test</button>

        {obj}

      </div>
    )
  }
}
