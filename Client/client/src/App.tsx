import React, { Component } from "react";

import Playlist from "./Playlist";

interface IProps {
    location: Location
}

interface IState {

}

export default class App extends Component<IProps, IState> {
    private code: string;
    private client: WebSocket;

    constructor (props: IProps) {
        super(props);
        this.code = props.location.search.slice(6);
        this.client = new WebSocket("ws://localhost:8081");
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
        }
        this.client.onmessage = (event) => {
            console.log("Message from server: " + event.data);
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
                    code={this.code}
                    client={this.client}
                    />
            </div>
        )
    }
}
