import React, { Component } from 'react';
import Highlight from 'react-fast-highlight';
import axios from 'axios';

import Paths from './Paths';

class Blob extends Component {
  constructor(props) {
    super(props);
    this.state = {
      doc: '',
    }
  }
  componentDidMount() {
    let that = this;
    axios.get('/ds/cat/' + this.props.match.params.id).then(resp => {
      that.setState({
        doc: resp.data
      });
    })
  }

  render() {
    return (
      <div>
        <Paths id={this.props.match.params.id}/>
        <Highlight languages={['c', 'xml', 'perl']}>
          {this.state.doc}
        </Highlight>
      </div>
    );
  }
}

export default Blob;
