import React, { Component } from "react";

import Playlist from "./Playlist";
import MutualPlaylists from "./MutualPlaylists";
import GroupsTab from "./GroupsTab";


interface IProps {
    location: Location
}

interface IState {
  playlist_data: string | null,
  group_data: string | null,
  readyState: number
}

export default class App extends Component<IProps, IState> {
    private code: string;
    private client: WebSocket;

    constructor (props: IProps) {
        super(props);
        this.state = {
          playlist_data: null,
          group_data: null,
          readyState: 0
        }
        this.code = props.location.search.slice(6);
        this.client = new WebSocket("ws://138.251.29.21:8081");
    }

    componentDidMount() {
        this.client.onopen = () => {
            console.log("Connection established");
            console.log("Sending to server");

            let authCodeMessage = {
                'type':'authCode',
                'code':this.code
            }
            this.client.send(JSON.stringify(authCodeMessage));
            this.setState({readyState:this.client.readyState});
        }
        this.client.onmessage = (event) => {
            console.log("Message from server: " + event.data);
            var response = JSON.parse(event.data);
            switch(response.type){
                case 'response_playlists':
                    console.log(response.data)
                    this.setState({
                        playlist_data: JSON.stringify(response.data)
                    })
                    break;
                case 'advertising_groups':
                console.log("HEHEHEHEHEHEHEHRERERERERERER")
                    console.log(response.data)
                    this.setState({group_data:response.data})
                    break;
                default:
            }
        }
        this.client.onclose = (event) => {
            if (event.wasClean) {
                console.log("Connection closed cleanly");
            } else {
                console.log("Connection died");
            }
        }
        this.client.onerror = (error: Event) => {
            console.log("Error: " + error.returnValue);
        }
    }

    render() {

      return(
          <div>
              <h1>{this.code}</h1>

              <Playlist
                  playlist_data={JSON.stringify({'data': this.state.playlist_data})}
                  code={this.code}
                  client={this.client}
                  />


              <MutualPlaylists
                  code={this.code}
                  client={this.client}
                  />


              {
                  this.state.readyState==1 ?
                  <GroupsTab
                      groups={JSON.stringify({'data': this.state.group_data})}
                      code={this.code}
                      client={this.client}
                      />
                      :
                  <div></div>
              }

          </div>
      )
    }
}
