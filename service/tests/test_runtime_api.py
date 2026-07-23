from fastapi.testclient import TestClient
import pytest

from multirepl_service.app import create_app


@pytest.fixture
def client(tmp_path):
    app = create_app(tmp_path / "runtimes.db")
    with TestClient(app) as test_client:
        yield test_client


def create_runtime(client, name="analysis"):
    response = client.post("/v1/runtimes", json={"name": name})
    assert response.status_code == 201
    return response.json()


def test_create_get_and_list_runtime(client):
    runtime = create_runtime(client)

    assert runtime["id"].startswith("rt_")
    assert runtime["name"] == "analysis"
    assert runtime["status"] == "idle"
    assert runtime["worker_generation"] == 0
    assert runtime["revision"] == 1
    assert runtime["environment"] == {
        "kind": "python",
        "executable": "python3",
        "digest": None,
    }

    get_response = client.get(f"/v1/runtimes/{runtime['id']}")
    assert get_response.status_code == 200
    assert get_response.json() == runtime

    list_response = client.get("/v1/runtimes")
    assert list_response.status_code == 200
    assert list_response.json()["items"] == [runtime]
    assert list_response.json()["total"] == 1


def test_update_runtime_increments_revision(client):
    runtime = create_runtime(client)

    response = client.patch(
        f"/v1/runtimes/{runtime['id']}",
        json={"name": "renamed", "status": "running"},
    )

    assert response.status_code == 200
    updated = response.json()
    assert updated["name"] == "renamed"
    assert updated["status"] == "running"
    assert updated["revision"] == 2
    assert updated["updated_at"] >= runtime["updated_at"]


def test_runtime_names_are_unique(client):
    create_runtime(client, "Analysis")

    response = client.post("/v1/runtimes", json={"name": "analysis"})

    assert response.status_code == 409
    assert response.json()["detail"] == "Runtime name already exists: analysis"


def test_list_filters_and_paginates(client):
    first = create_runtime(client, "first")
    second = create_runtime(client, "second")
    client.patch(f"/v1/runtimes/{second['id']}", json={"status": "running"})

    filtered = client.get("/v1/runtimes", params={"status": "running"})
    assert filtered.status_code == 200
    assert [item["name"] for item in filtered.json()["items"]] == ["second"]
    assert filtered.json()["total"] == 1

    paginated = client.get("/v1/runtimes", params={"limit": 1, "offset": 1})
    assert paginated.status_code == 200
    assert paginated.json()["total"] == 2
    assert paginated.json()["items"][0]["id"] == second["id"]
    assert paginated.json()["items"][0]["id"] != first["id"]


def test_delete_runtime(client):
    runtime = create_runtime(client)

    delete_response = client.delete(f"/v1/runtimes/{runtime['id']}")
    assert delete_response.status_code == 204
    assert delete_response.content == b""

    get_response = client.get(f"/v1/runtimes/{runtime['id']}")
    assert get_response.status_code == 404

    second_delete = client.delete(f"/v1/runtimes/{runtime['id']}")
    assert second_delete.status_code == 404


def test_runtime_persists_across_app_instances(tmp_path):
    database_path = tmp_path / "persistent.db"
    first_app = create_app(database_path)
    with TestClient(first_app) as first_client:
        runtime = create_runtime(first_client)

    second_app = create_app(database_path)
    with TestClient(second_app) as second_client:
        response = second_client.get(f"/v1/runtimes/{runtime['id']}")
        assert response.status_code == 200
        assert response.json()["name"] == "analysis"


def test_openapi_schema_is_small_and_explicit(client):
    response = client.get("/openapi.json")
    assert response.status_code == 200
    schema = response.json()

    assert schema["info"]["title"] == "Multirepl Runtime API"
    assert set(schema["paths"]) == {
        "/v1/runtimes",
        "/v1/runtimes/{runtime_id}",
    }
    assert schema["paths"]["/v1/runtimes"]["post"]["operationId"] == "createRuntime"
    assert schema["paths"]["/v1/runtimes/{runtime_id}"]["delete"]["operationId"] == "deleteRuntime"
