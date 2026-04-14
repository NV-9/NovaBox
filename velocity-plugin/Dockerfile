FROM maven:3.9-eclipse-temurin-17 AS build
WORKDIR /build
COPY pom.xml .
RUN mvn dependency:go-offline -q
COPY src ./src
RUN mvn package -q -DskipTests

FROM scratch AS export
COPY --from=build /build/target/novabox-velocity-1.0.0.jar /novabox-velocity.jar
