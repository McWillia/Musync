import React from "react";

export default class App extends React.Component {
    constructor (props) {
        super(props);
        this.code = props.location.search.slice(6);
        this.client = new WebSocket("ws://localhost:8081");
    }

    componentDidMount() {
        this.client.onopen = () => {
            console.log("Connection established");
            console.log("Sending to server");
            this.client.send(this.code);
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
        this.client.onerror = (error) => {
            console.log("Error: " + error.message);
        }
    }

    render() {
        
        return(
            <h1>{this.code}</h1>
        )
    }
}
