import { symbol } from './another-package';

export const functionHelloWorld = () => {
  console.log("hello")
};

export const constantMyConstant = "standa";


const f = function() {
  helloWorld();
  console.log(constantMyConstant);

  console.log(symbol);
}

const f2 = () => {
  const x = "hello";
}

class ClassHelloWorld {
  constructor() {
    helloWorld();
    console.log(constantMyConstant);

    const symbol = 1;

    console.log(symbol);
  }
}
