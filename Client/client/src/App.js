import React from "react";

export default class App extends React.Component {
    constructor (props) {
        super(props);
        this.code = props.location.search.slice(6);
    }

    

    render() {
        
        return(
            <h1>{this.code}</h1>
        )
    }
}
