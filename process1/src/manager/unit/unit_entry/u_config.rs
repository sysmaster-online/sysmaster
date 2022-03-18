struct Conf(String,String);


impl Conf{
    fn new(key:String,value:String) -> Self{
        Self(key,value)
    }
    fn get_key(&self) ->&str{
        &self.0
    }
    fn get_values(&self)->Vec<&str>{
        let values = self.1.split_whitespace();
        let mut vs: Vec<&str> = Vec::new();
        for v in values{
            vs.push(v);
        }
        vs
    }
}

struct Section<Conf>(String,Vec<Conf>);

impl Section<Conf>{
    fn new(name:String) -> Self{
        Section(name,Vec::new())
    }

    fn getSectionName(&self) -> &str{
        &self.0
    }
    fn getConfs(&self) -> &Vec<Conf>{
        &self.1
    }
    fn addConf(&mut self,conf:Conf){
        self.1.push(conf);
    }
}
struct  Confs<T>{
    ctype:String,
    sections:Vec<T>,
}


impl Confs<Section<Conf>> {
    /** need type ownership**/
    fn new(ctype:String) -> Self {
        Self{
            ctype:String::from(""), 
            sections:Vec::new(),
        }
    }

    pub  fn getCtypes(&self) -> &str {
        &self.ctype
    }

    pub fn registerSection(&mut self, section:Section<Conf>){
        self.sections.push(section);
    }

    pub fn getSections(&self) -> &Vec<Section<Conf>>{
       &self.sections
    }
}



trait ConfFactory{
    fn productConf(&self) -> Option<Confs<Section<Conf>>>;
}