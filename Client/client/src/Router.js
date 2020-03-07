import React from "react";
import { BrowserRouter, Switch, Route } from "react-router-dom";
import Login from "./Login"
import App from "./App"

export default function Router() {
    return (
        <BrowserRouter>
            <div>
                <Switch>
                    <Route path="/home" component={App}>
                    </Route>
                    <Route path="/" component={Login}>
                    </Route>
                </Switch>
            </div>
        </BrowserRouter>
    )
}